//! Bounded Responses SSE transport. Previews are provisional; chat::parse validates completion.
use crate::cloud_api::ApiError;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};

fn interrupted() -> ApiError {
    ApiError {
        message: "The streamed answer was interrupted; its outcome is uncertain and may be billable. Nothing was retried automatically.".into(),
        ambiguous: true, retry: false, missing: false, delay: 0,
    }
}

pub fn read(reader: impl Read, preview: &mut dyn FnMut(&str)) -> Result<Value, ApiError> {
    let mut reader = BufReader::new(reader.take(8 * 1024 * 1024 + 1));
    let mut line = String::new();
    let mut data = String::new();
    let mut text = String::new();
    let mut total = 0;
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|_| interrupted())?;
        total += bytes;
        if bytes == 0 || total > 8 * 1024 * 1024 {
            return Err(interrupted());
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(value) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(value.strip_prefix(' ').unwrap_or(value));
        } else if line.is_empty() && !data.is_empty() {
            let event: Value = serde_json::from_str(&data).map_err(|_| interrupted())?;
            data.clear();
            match event["type"].as_str() {
                Some("response.output_text.delta") => {
                    text.push_str(event["delta"].as_str().ok_or_else(interrupted)?);
                    if text.len() > 32000 {
                        return Err(interrupted());
                    }
                    if let Some(answer) = partial_answer(&text) {
                        preview(&answer);
                    }
                }
                Some("response.completed") => return Ok(event["response"].clone()),
                Some("response.failed" | "response.incomplete" | "error") => {
                    return Err(interrupted())
                }
                _ => {}
            }
        }
    }
}

// Decode only the JSON answer string, never expose schema syntax/source arrays.
fn partial_answer(text: &str) -> Option<String> {
    static PATTERN: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r#"^\s*\{\s*"answer"\s*:\s*""#).expect("literal answer pattern")
    });
    let start = PATTERN.find(text)?.end() - 1;
    let value = &text[start..];
    let mut escaped = false;
    for (i, c) in value.char_indices().skip(1) {
        if !escaped && c == '"' {
            return serde_json::from_str(&value[..=i]).ok();
        }
        escaped = !escaped && c == '\\';
    }
    // A delta may end inside an escape or a UTF-16 surrogate pair.
    for end in value
        .char_indices()
        .map(|(i, _)| i)
        .chain([value.len()])
        .rev()
    {
        if end == 0 {
            break;
        }
        if let Ok(decoded) = serde_json::from_str::<String>(&format!("{}\"", &value[..end])) {
            return Some(decoded);
        }
    }
    None
}

pub fn safe_preview(text: &str, key: &str) -> String {
    let mut end = text.find("sk-").unwrap_or(text.len());
    if !key.is_empty() {
        if let Some(i) = text.find(key) {
            end = end.min(i);
        }
        // Hold any trailing prefix of the saved credential until its identity is clear.
        for (i, _) in text.char_indices() {
            if key.starts_with(&text[i..]) {
                end = end.min(i);
                break;
            }
        }
    }
    // Also hold a split sk- marker across deltas.
    for suffix in ["s", "sk"] {
        if text.ends_with(suffix) {
            end = end.min(text.len() - suffix.len());
        }
    }
    text[..end].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn event(value: Value) -> String {
        format!("event: ignored\r\ndata: {value}\r\n\r\n")
    }
    #[test]
    fn streams_decoded_answer_before_completed_response_and_keeps_usage() {
        let answer = json!({"answer":"Hello\n\"world\" 🦀 [1].","sources":[1]}).to_string();
        let mut wire = String::from(": heartbeat\n\n");
        for c in answer.chars() {
            wire += &event(json!({"type":"response.output_text.delta","delta":c.to_string()}));
        }
        let completed = json!({"status":"completed","usage":{"input_tokens":42},"output":[]});
        wire += &event(json!({"type":"response.completed","response":completed}));
        let mut previews = vec![];
        let result = read(wire.as_bytes(), &mut |s| previews.push(s.to_owned())).unwrap();
        assert_eq!(result, completed);
        assert!(previews.iter().any(|p| p == "Hello"));
        assert_eq!(previews.last().unwrap(), "Hello\n\"world\" 🦀 [1].");
        assert!(!previews.iter().any(|p| p.contains("sources")));
        assert_eq!(
            partial_answer(r#"{"answer":"\uD83E\uDD80"#),
            Some("🦀".into())
        );
    }
    #[test]
    fn interrupted_failed_malformed_and_oversized_streams_are_uncertain_without_retry() {
        for wire in [
            event(json!({"type":"response.output_text.delta","delta":"{\"answer\":\"Partial"})),
            event(json!({"type":"response.failed"})),
            event(json!({"type":"response.incomplete"})),
            "data: not-json\n\n".into(),
            event(json!({"type":"response.output_text.delta","delta":"a".repeat(32001)})),
        ] {
            let err = read(wire.as_bytes(), &mut |_| {}).unwrap_err();
            assert!(err.ambiguous);
            assert!(!err.retry);
        }
    }
    #[test]
    fn provisional_text_holds_split_credentials_before_they_can_be_displayed() {
        let key = "my-synthetic-secret";
        for n in 1..=key.len() {
            assert_eq!(
                safe_preview(
                    &format!("Answer {key}")
                        .chars()
                        .take(7 + n)
                        .collect::<String>(),
                    key
                ),
                "Answer "
            );
        }
        assert_eq!(safe_preview("Answer sk-secret-more", key), "Answer ");
        assert_eq!(safe_preview("Answer sk", key), "Answer ");
        assert_eq!(safe_preview("Answer s", key), "Answer ");
        assert_eq!(safe_preview("Answer complete.", key), "Answer complete.");
    }
}
