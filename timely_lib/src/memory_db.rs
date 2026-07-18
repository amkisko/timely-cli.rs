use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, named_params};
use serde::Serialize;

#[derive(Serialize)]
pub struct MemoryStatus {
    db_path: String,
    entry_count: i64,
    app_count: i64,
    privacy_rule_count: i64,
    first_captured_at_utc: Option<String>,
    last_captured_at_utc: Option<String>,
}

#[derive(Serialize)]
pub struct MemoryApp {
    app_name: String,
    entry_count: i64,
    last_captured_at_utc: Option<String>,
    icon_path: Option<String>,
    seen_at: Option<String>,
}

#[derive(Serialize)]
pub struct MemoryEntry {
    id: String,
    captured_at_utc: String,
    captured_at_with_tz: String,
    app_name: String,
    window_title: String,
    details: Option<String>,
    uploaded: bool,
    rewritten: bool,
}

pub fn status(db_path: Option<&str>) -> Result<MemoryStatus> {
    let (conn, db_path) = open(db_path)?;
    let (entry_count, first_captured_at_utc, last_captured_at_utc): (
        i64,
        Option<String>,
        Option<String>,
    ) = conn.query_row(
        "SELECT COUNT(*), MIN(captured_at_utc), MAX(captured_at_utc) FROM captured_entries",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let app_count = scalar(&conn, "SELECT COUNT(*) FROM seen_apps")?;
    let privacy_rule_count = scalar(&conn, "SELECT COUNT(*) FROM privacy_rules")?;
    Ok(MemoryStatus {
        db_path,
        entry_count,
        app_count,
        privacy_rule_count,
        first_captured_at_utc,
        last_captured_at_utc,
    })
}

pub fn list_apps(db_path: Option<&str>, limit: usize) -> Result<Vec<MemoryApp>> {
    let (conn, _) = open(db_path)?;
    // Ceiling: LIMIT bounds returned apps, not GROUP BY work over captured_entries.
    // Upgrade path: summary table or indexed counts when large Memory DBs matter.
    let mut stmt = conn.prepare(
        "SELECT ce.app_name, COUNT(*) AS entry_count, MAX(ce.captured_at_utc), sa.icon_path, sa.seen_at
         FROM captured_entries ce
         LEFT JOIN seen_apps sa ON sa.app_name = ce.app_name
         GROUP BY ce.app_name, sa.icon_path, sa.seen_at
         ORDER BY entry_count DESC, ce.app_name ASC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map([normalized_limit(limit) as i64], |row| {
        Ok(MemoryApp {
            app_name: row.get(0)?,
            entry_count: row.get(1)?,
            last_captured_at_utc: row.get(2)?,
            icon_path: row.get(3)?,
            seen_at: row.get(4)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn recent_entries(
    db_path: Option<&str>,
    app: Option<&str>,
    limit: usize,
    include_details: bool,
) -> Result<Vec<MemoryEntry>> {
    let (conn, _) = open(db_path)?;
    let details = if include_details {
        "details"
    } else {
        "NULL AS details"
    };
    let sql = format!(
        "SELECT id, captured_at_utc, captured_at_with_tz, app_name, window_title, {details}, uploaded, rewritten
         FROM captured_entries
         WHERE (:app IS NULL OR app_name = :app)
         ORDER BY captured_at_utc DESC
         LIMIT :limit"
    );
    query_entries(&conn, &sql, app, None, limit)
}

pub fn search_entries(
    db_path: Option<&str>,
    query: &str,
    app: Option<&str>,
    limit: usize,
    include_details: bool,
) -> Result<Vec<MemoryEntry>> {
    let (conn, _) = open(db_path)?;
    let details = if include_details {
        "details"
    } else {
        "NULL AS details"
    };
    let pattern = format!("%{}%", escape_like(query));
    let sql = format!(
        "SELECT id, captured_at_utc, captured_at_with_tz, app_name, window_title, {details}, uploaded, rewritten
         FROM captured_entries
         WHERE (:app IS NULL OR app_name = :app)
           AND (window_title LIKE :pattern ESCAPE '\\' OR COALESCE(details, '') LIKE :pattern ESCAPE '\\')
         ORDER BY captured_at_utc DESC
         LIMIT :limit"
    );
    query_entries(&conn, &sql, app, Some(&pattern), limit)
}

pub fn export_entries(
    db_path: Option<&str>,
    app: Option<&str>,
    since: Option<&str>,
    upto: Option<&str>,
    limit: usize,
    include_details: bool,
) -> Result<Vec<MemoryEntry>> {
    let (conn, _) = open(db_path)?;
    let details = if include_details {
        "details"
    } else {
        "NULL AS details"
    };
    let sql = format!(
        "SELECT id, captured_at_utc, captured_at_with_tz, app_name, window_title, {details}, uploaded, rewritten
         FROM captured_entries
         WHERE (:app IS NULL OR app_name = :app)
           AND (:since IS NULL OR captured_at_utc >= :since)
           AND (:upto IS NULL OR captured_at_utc <= :upto)
         ORDER BY captured_at_utc ASC
         LIMIT :limit"
    );
    let mut stmt = conn.prepare(&sql)?;
    let limit = normalized_export_limit(limit) as i64;
    let rows = stmt.query_map(
        named_params! {
            ":app": app,
            ":since": since,
            ":upto": upto,
            ":limit": limit,
        },
        map_entry,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn normalized_limit(limit: usize) -> usize {
    limit.clamp(1, 200)
}

pub fn normalized_export_limit(limit: usize) -> usize {
    limit.clamp(1, 10_000)
}

fn query_entries(
    conn: &Connection,
    sql: &str,
    app: Option<&str>,
    pattern: Option<&str>,
    limit: usize,
) -> Result<Vec<MemoryEntry>> {
    let mut stmt = conn.prepare(sql)?;
    let limit = normalized_limit(limit) as i64;
    let rows = match pattern {
        Some(pattern) => stmt.query_map(
            named_params! {
                ":app": app,
                ":pattern": pattern,
                ":limit": limit,
            },
            map_entry,
        )?,
        None => stmt.query_map(
            named_params! {
                ":app": app,
                ":limit": limit,
            },
            map_entry,
        )?,
    };
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryEntry> {
    Ok(MemoryEntry {
        id: row.get(0)?,
        captured_at_utc: row.get(1)?,
        captured_at_with_tz: row.get(2)?,
        app_name: row.get(3)?,
        window_title: row.get(4)?,
        details: row.get(5)?,
        uploaded: row.get(6)?,
        rewritten: row.get(7)?,
    })
}

fn open(db_path: Option<&str>) -> Result<(Connection, String)> {
    let path = match db_path {
        Some(db_path) => PathBuf::from(db_path),
        None => default_db_path()?,
    };
    if !path.exists() {
        bail!("Memory database not found at {}", path.display());
    }
    let display_path = path.display().to_string();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open {}", path.display()))?;
    Ok((conn, display_path))
}

fn default_db_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("TIMELY_MEMORY_DB")
        && !path.trim().is_empty()
    {
        return Ok(PathBuf::from(path));
    }
    let home = env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join("Library/Application Support/com.TimelyApp.Memory/db.sqlite"))
}

fn scalar(conn: &Connection, sql: &str) -> Result<i64> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn escape_like(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_like_wildcards() {
        assert_eq!(escape_like("100%_done\\"), "100\\%\\_done\\\\");
    }

    #[test]
    fn clamps_limits() {
        assert_eq!(normalized_limit(0), 1);
        assert_eq!(normalized_limit(400), 200);
        assert_eq!(normalized_export_limit(0), 1);
        assert_eq!(normalized_export_limit(50_000), 10_000);
    }
}

#[cfg(test)]
#[path = "memory_export_tests.rs"]
mod export_tests;
