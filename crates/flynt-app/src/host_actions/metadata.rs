//! HostAction metadata extraction helpers.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct HostActionCandidate {
    pub id: String,
    pub action_type: String,
    pub params: Value,
    pub raw: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HostActionOutcomeView {
    pub action_id: String,
    pub status: String,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub raw: Value,
}

pub fn extract_host_actions(value: Option<&Value>) -> Vec<HostActionCandidate> {
    let Some(value) = value else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_actions(value, &mut out);
    out
}

pub fn extract_host_action_outcomes(value: Option<&Value>) -> Vec<HostActionOutcomeView> {
    let Some(value) = value else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_outcomes(value, &mut out);
    out
}

fn collect_actions(value: &Value, out: &mut Vec<HostActionCandidate>) {
    match value {
        Value::Object(obj) => {
            if let Some(actions) = obj
                .get("actions")
                .or_else(|| obj.get("host_actions"))
                .or_else(|| obj.get("hostActions"))
                .and_then(Value::as_array)
            {
                out.extend(actions.iter().filter_map(parse_action));
            }
            if let Some(actions) = obj.get("omegon/hostActions").and_then(Value::as_array) {
                out.extend(actions.iter().filter_map(parse_action));
            }
            if let Some(meta) = obj.get("_meta") {
                collect_actions(meta, out);
            }
            if let Some(details) = obj.get("details") {
                collect_actions(details, out);
            }
            if let Some(output) = obj.get("output") {
                collect_actions(output, out);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_actions(value, out);
            }
        }
        _ => {}
    }
}

fn collect_outcomes(value: &Value, out: &mut Vec<HostActionOutcomeView>) {
    match value {
        Value::Object(obj) => {
            if let Some(outcomes) = obj
                .get("host_action_outcomes")
                .or_else(|| obj.get("hostActionOutcomes"))
                .and_then(Value::as_array)
            {
                out.extend(outcomes.iter().filter_map(parse_outcome));
            }
            if let Some(meta) = obj.get("_meta") {
                collect_outcomes(meta, out);
            }
            if let Some(details) = obj.get("details") {
                collect_outcomes(details, out);
            }
            if let Some(output) = obj.get("output") {
                collect_outcomes(output, out);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_outcomes(value, out);
            }
        }
        _ => {}
    }
}

fn parse_action(value: &Value) -> Option<HostActionCandidate> {
    let obj = value.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();
    let action_type = obj
        .get("type")
        .or_else(|| obj.get("action_type"))
        .or_else(|| obj.get("actionType"))?
        .as_str()?
        .to_string();
    let params = obj.get("params").cloned().unwrap_or(Value::Null);
    Some(HostActionCandidate {
        id,
        action_type,
        params,
        raw: value.clone(),
    })
}

fn parse_outcome(value: &Value) -> Option<HostActionOutcomeView> {
    let obj = value.as_object()?;
    let action_id = obj
        .get("action_id")
        .or_else(|| obj.get("actionId"))?
        .as_str()?
        .to_string();
    let status = obj.get("status")?.as_str()?.to_string();
    Some(HostActionOutcomeView {
        action_id,
        status,
        result: obj.get("result").cloned(),
        error: obj.get("error").cloned(),
        raw: value.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_host_actions_from_details() {
        let value = json!({
            "details": {
                "actions": [{
                    "id": "open-reader",
                    "type": "terminal.create@1",
                    "params": {"command": "bookokrat"}
                }]
            }
        });
        let actions = extract_host_actions(Some(&value));
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, "terminal.create@1");
    }

    #[test]
    fn extracts_host_actions_from_meta() {
        let value = json!({
            "_meta": {
                "omegon/hostActions": [{
                    "id": "open-reader",
                    "type": "terminal.create@1",
                    "params": {"command": "bookokrat"}
                }]
            }
        });
        let actions = extract_host_actions(Some(&value));
        assert_eq!(actions[0].id, "open-reader");
    }

    #[test]
    fn extracts_host_action_outcomes() {
        let value = json!({
            "details": {
                "host_action_outcomes": [{
                    "action_id": "open-reader",
                    "status": "completed",
                    "result": {"terminal_id": "term-1"}
                }]
            }
        });
        let outcomes = extract_host_action_outcomes(Some(&value));
        assert_eq!(outcomes[0].status, "completed");
        assert_eq!(
            outcomes[0].result.as_ref().unwrap()["terminal_id"],
            "term-1"
        );
    }
}
