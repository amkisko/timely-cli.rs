//! Post-command hints for discoverable next steps.

use serde_json::Value;

use crate::api_cli::{
    ApiSubcommand, ResourceCommand, ResourceSubcommand, TimeEntriesCommand, TimeEntriesSubcommand,
};
use crate::api_cli_extra::{
    PermissionsCommand, PermissionsSubcommand, ProjectCommand, ProjectSubcommand, TeamCommand,
    TeamSubcommand,
};
use crate::cli_util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListHint {
    Me,
    Clients,
    Teams,
    Projects,
    Users,
    Labels,
    Tasks,
    TimeEntries,
}

pub fn list_hint(command: &ApiSubcommand) -> Option<ListHint> {
    match command {
        ApiSubcommand::Me(_) => Some(ListHint::Me),
        ApiSubcommand::Clients(ResourceCommand {
            command: ResourceSubcommand::List(_),
        }) => Some(ListHint::Clients),
        ApiSubcommand::Teams(TeamCommand {
            command: TeamSubcommand::List(_),
        }) => Some(ListHint::Teams),
        ApiSubcommand::Projects(ProjectCommand {
            command: ProjectSubcommand::List(_),
        }) => Some(ListHint::Projects),
        ApiSubcommand::Users(ResourceCommand {
            command: ResourceSubcommand::List(_),
        }) => Some(ListHint::Users),
        ApiSubcommand::Labels(ResourceCommand {
            command: ResourceSubcommand::List(_),
        }) => Some(ListHint::Labels),
        ApiSubcommand::Tasks(ResourceCommand {
            command: ResourceSubcommand::List(_),
        }) => Some(ListHint::Tasks),
        ApiSubcommand::TimeEntries(TimeEntriesCommand {
            command: TimeEntriesSubcommand::List(_),
        }) => Some(ListHint::TimeEntries),
        ApiSubcommand::Permissions(PermissionsCommand {
            command: PermissionsSubcommand::Current(_),
        }) => None,
        _ => None,
    }
}

pub fn maybe_suggest(hint: ListHint, value: &Value, quiet: bool) {
    if quiet {
        return;
    }
    let Some(message) = suggest_message(hint, value) else {
        return;
    };
    cli_util::suggest_next_command(quiet, &message);
}

fn suggest_message(hint: ListHint, value: &Value) -> Option<String> {
    match hint {
        ListHint::Me => Some("Try: timely api clients list".to_string()),
        ListHint::Clients => {
            first_numeric_id(value).map(|id| format!("Try: timely api clients get {id}"))
        }
        ListHint::Teams => {
            first_numeric_id(value).map(|id| format!("Try: timely api teams get {id}"))
        }
        ListHint::Projects => {
            first_numeric_id(value).map(|id| format!("Try: timely api projects get {id}"))
        }
        ListHint::Users => {
            first_numeric_id(value).map(|id| format!("Try: timely api users get {id}"))
        }
        ListHint::Labels => {
            first_numeric_id(value).map(|id| format!("Try: timely api labels get {id}"))
        }
        ListHint::Tasks => {
            first_numeric_id(value).map(|id| format!("Try: timely api tasks get {id}"))
        }
        ListHint::TimeEntries => {
            first_numeric_id(value).map(|id| format!("Try: timely api time-entries get {id}"))
        }
    }
}

fn first_numeric_id(value: &Value) -> Option<i64> {
    list_items(value)?.first()?.get("id")?.as_i64()
}

fn list_items(value: &Value) -> Option<&Vec<Value>> {
    value
        .as_array()
        .or_else(|| value.get("data").and_then(Value::as_array))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_first_id_from_array() {
        let value = serde_json::json!([{"id": 42}, {"id": 7}]);
        assert_eq!(first_numeric_id(&value), Some(42));
    }

    #[test]
    fn extracts_first_id_from_data_wrapper() {
        let value = serde_json::json!({"data": [{"id": 9}]});
        assert_eq!(first_numeric_id(&value), Some(9));
    }

    #[test]
    fn me_hint_does_not_need_rows() {
        assert_eq!(
            suggest_message(ListHint::Me, &serde_json::json!([])),
            Some("Try: timely api clients list".to_string())
        );
    }

    #[test]
    fn clients_hint_uses_first_row() {
        let value = serde_json::json!([{"id": 5}]);
        assert_eq!(
            suggest_message(ListHint::Clients, &value).as_deref(),
            Some("Try: timely api clients get 5")
        );
    }
}
