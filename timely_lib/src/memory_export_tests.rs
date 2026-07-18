use super::*;
use rusqlite::params;

#[test]
fn export_entries_filters_orders_and_clamps() {
    let path = std::env::temp_dir().join(format!(
        "timely-memory-export-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE captured_entries (
                id TEXT PRIMARY KEY,
                captured_at_utc TEXT NOT NULL,
                captured_at_with_tz TEXT NOT NULL,
                app_name TEXT NOT NULL,
                window_title TEXT NOT NULL,
                details TEXT,
                uploaded INTEGER NOT NULL,
                rewritten INTEGER NOT NULL
            );",
        )
        .unwrap();
        for (id, at, app, title) in [
            ("1", "2026-01-01T10:00:00Z", "Code", "a"),
            ("2", "2026-01-02T10:00:00Z", "Code", "b"),
            ("3", "2026-01-03T10:00:00Z", "Browser", "c"),
            ("4", "2026-01-04T10:00:00Z", "Code", "d"),
        ] {
            conn.execute(
                "INSERT INTO captured_entries VALUES (?1, ?2, ?2, ?3, ?4, 'detail', 0, 0)",
                params![id, at, app, title],
            )
            .unwrap();
        }
    }
    let path_str = path.to_str().unwrap();
    let filtered = export_entries(
        Some(path_str),
        Some("Code"),
        Some("2026-01-02T00:00:00Z"),
        Some("2026-01-03T23:59:59Z"),
        100,
        true,
    )
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "2");
    assert_eq!(filtered[0].details.as_deref(), Some("detail"));

    let ordered = export_entries(Some(path_str), None, None, None, 10, false).unwrap();
    let ids: Vec<_> = ordered.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(ids, vec!["1", "2", "3", "4"]);
    assert!(ordered.iter().all(|entry| entry.details.is_none()));

    let limited = export_entries(Some(path_str), None, None, None, 2, false).unwrap();
    assert_eq!(limited.len(), 2);
    assert_eq!(limited[0].id, "1");
    assert_eq!(limited[1].id, "2");

    let _ = std::fs::remove_file(&path);
}
