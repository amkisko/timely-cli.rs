use anyhow::{Result, anyhow};
use serde_json::Value;

pub const OPENAPI_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/openapi.json"));

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    pub method: String,
    pub path: String,
    pub summary: String,
    pub tags: Vec<String>,
}

pub fn spec() -> Result<Value> {
    serde_json::from_str(OPENAPI_JSON).map_err(Into::into)
}

pub fn operations() -> Result<Vec<Operation>> {
    let spec = spec()?;
    let paths = spec
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("OpenAPI document does not contain paths"))?;
    let mut out = Vec::new();

    for (path, item) in paths {
        let Some(item) = item.as_object() else {
            continue;
        };
        for method in ["get", "post", "put", "patch", "delete"] {
            let Some(op) = item.get(method) else {
                continue;
            };
            if !op.is_object() {
                continue;
            }
            out.push(Operation {
                id: operation_id(method, path, op),
                method: method.to_uppercase(),
                path: path.to_string(),
                summary: op
                    .get("summary")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                tags: tags(op),
            });
        }
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn operation_id(method: &str, path: &str, op: &Value) -> String {
    op.get("operationId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| generated_operation_id(method, path))
}

fn tags(op: &Value) -> Vec<String> {
    op.get("tags")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn generated_operation_id(method: &str, path: &str) -> String {
    let suffix = path
        .trim_matches('/')
        .replace('{', "by_")
        .replace('}', "")
        .replace(['/', '-'], "_")
        .replace('.', "_");
    format!("{method}_{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMUM_EMBEDDED_OPERATIONS: usize = 20;

    #[test]
    fn extracted_spec_has_expected_shape() {
        let spec = spec().unwrap();
        assert_eq!(spec["openapi"], "3.1.0");
        assert_eq!(spec["info"]["title"], "Timely API Docs");
        let paths = spec
            .get("paths")
            .and_then(Value::as_object)
            .expect("paths object");
        assert!(
            !paths.is_empty(),
            "embedded OpenAPI paths must not be empty; restore tmp/openapi/openapi.json"
        );
    }

    #[test]
    fn operations_are_sorted() {
        let ops = operations().unwrap();
        assert!(ops.windows(2).all(|pair| pair[0].id <= pair[1].id));
    }

    #[test]
    fn embedded_spec_includes_a_usable_operation_set() {
        let ops = operations().unwrap();
        assert!(
            ops.len() >= MINIMUM_EMBEDDED_OPERATIONS,
            "expected at least {MINIMUM_EMBEDDED_OPERATIONS} embedded operations, got {}",
            ops.len()
        );
        assert!(
            ops.iter().any(|operation| {
                operation.path.contains("{account_id}") || operation.path.contains("user_accounts")
            }),
            "embedded OpenAPI should include account-scoped paths"
        );
    }
}
