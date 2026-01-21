//! Message parser for Claude Code SDK responses.

use crate::types::{
    AssistantMessage, ContentBlock, Error, Message, MessageContent, Result, ResultMessage,
    StreamEvent, SystemMessage, UserMessage,
};

/// Parse message from CLI output into typed Message objects.
///
/// # Arguments
/// * `data` - Raw message JSON value from CLI output
///
/// # Returns
/// Parsed Message object
///
/// # Errors
/// Returns `Error::MessageParse` if parsing fails or message type is unrecognized
pub fn parse_message(data: serde_json::Value) -> Result<Message> {
    let obj = data
        .as_object()
        .ok_or_else(|| Error::MessageParse("Expected JSON object".to_string()))?;

    let message_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MessageParse("Missing 'type' field".to_string()))?;

    match message_type {
        "user" => parse_user_message(data),
        "assistant" => parse_assistant_message(data),
        "system" => parse_system_message(data),
        "result" => parse_result_message(data),
        "stream_event" => parse_stream_event(data),
        _ => Err(Error::MessageParse(format!(
            "Unknown message type: {}",
            message_type
        ))),
    }
}

fn parse_user_message(data: serde_json::Value) -> Result<Message> {
    let obj = data.as_object().unwrap();

    let parent_tool_use_id = obj
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let uuid = obj
        .get("uuid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let message_obj = obj
        .get("message")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::MessageParse("Missing 'message' field".to_string()))?;

    let content_value = message_obj
        .get("content")
        .ok_or_else(|| Error::MessageParse("Missing 'content' field".to_string()))?;

    let content = if let Some(content_str) = content_value.as_str() {
        MessageContent::String(content_str.to_string())
    } else if let Some(content_array) = content_value.as_array() {
        let blocks = parse_content_blocks(content_array)?;
        MessageContent::Blocks(blocks)
    } else {
        return Err(Error::MessageParse(
            "Invalid content format".to_string(),
        ));
    };

    Ok(Message::User(UserMessage {
        content,
        uuid,
        parent_tool_use_id,
    }))
}

fn parse_assistant_message(data: serde_json::Value) -> Result<Message> {
    let obj = data.as_object().unwrap();

    let parent_tool_use_id = obj
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let message_obj = obj
        .get("message")
        .and_then(|v| v.as_object())
        .ok_or_else(|| Error::MessageParse("Missing 'message' field".to_string()))?;

    let model = message_obj
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MessageParse("Missing 'model' field".to_string()))?
        .to_string();

    let content_array = message_obj
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::MessageParse("Missing 'content' array".to_string()))?;

    let content = parse_content_blocks(content_array)?;

    let error = message_obj
        .get("error")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    Ok(Message::Assistant(AssistantMessage {
        content,
        model,
        parent_tool_use_id,
        error,
    }))
}

fn parse_system_message(data: serde_json::Value) -> Result<Message> {
    let obj = data.as_object().unwrap();

    let subtype = obj
        .get("subtype")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MessageParse("Missing 'subtype' field".to_string()))?
        .to_string();

    Ok(Message::System(SystemMessage {
        subtype,
        data: data.clone(),
    }))
}

fn parse_result_message(data: serde_json::Value) -> Result<Message> {
    let obj = data.as_object().unwrap();

    let subtype = obj
        .get("subtype")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MessageParse("Missing 'subtype' field".to_string()))?
        .to_string();

    let duration_ms = obj
        .get("duration_ms")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Error::MessageParse("Missing 'duration_ms' field".to_string()))?;

    let duration_api_ms = obj
        .get("duration_api_ms")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Error::MessageParse("Missing 'duration_api_ms' field".to_string()))?;

    let is_error = obj
        .get("is_error")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| Error::MessageParse("Missing 'is_error' field".to_string()))?;

    let num_turns = obj
        .get("num_turns")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| Error::MessageParse("Missing 'num_turns' field".to_string()))? as i32;

    let session_id = obj
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MessageParse("Missing 'session_id' field".to_string()))?
        .to_string();

    let total_cost_usd = obj.get("total_cost_usd").and_then(|v| v.as_f64());

    let usage = obj.get("usage").cloned();

    let result = obj
        .get("result")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let structured_output = obj.get("structured_output").cloned();

    Ok(Message::Result(ResultMessage {
        subtype,
        duration_ms,
        duration_api_ms,
        is_error,
        num_turns,
        session_id,
        total_cost_usd,
        usage,
        result,
        structured_output,
    }))
}

fn parse_stream_event(data: serde_json::Value) -> Result<Message> {
    let obj = data.as_object().unwrap();

    let uuid = obj
        .get("uuid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MessageParse("Missing 'uuid' field".to_string()))?
        .to_string();

    let session_id = obj
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::MessageParse("Missing 'session_id' field".to_string()))?
        .to_string();

    let event = obj
        .get("event")
        .ok_or_else(|| Error::MessageParse("Missing 'event' field".to_string()))?
        .clone();

    let parent_tool_use_id = obj
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(Message::Stream(StreamEvent {
        uuid,
        session_id,
        event,
        parent_tool_use_id,
    }))
}

fn parse_content_blocks(blocks: &[serde_json::Value]) -> Result<Vec<ContentBlock>> {
    let mut result = Vec::new();

    for block in blocks {
        let block_obj = block
            .as_object()
            .ok_or_else(|| Error::MessageParse("Content block must be an object".to_string()))?;

        let block_type = block_obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::MessageParse("Missing 'type' in content block".to_string()))?;

        let content_block = match block_type {
            "text" => {
                let text = block_obj
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::MessageParse("Missing 'text' field".to_string()))?
                    .to_string();
                ContentBlock::Text { text }
            }
            "thinking" => {
                let thinking = block_obj
                    .get("thinking")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::MessageParse("Missing 'thinking' field".to_string()))?
                    .to_string();
                let signature = block_obj
                    .get("signature")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::MessageParse("Missing 'signature' field".to_string()))?
                    .to_string();
                ContentBlock::Thinking { thinking, signature }
            }
            "tool_use" => {
                let id = block_obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::MessageParse("Missing 'id' field".to_string()))?
                    .to_string();
                let name = block_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::MessageParse("Missing 'name' field".to_string()))?
                    .to_string();
                let input = block_obj
                    .get("input")
                    .ok_or_else(|| Error::MessageParse("Missing 'input' field".to_string()))?
                    .clone();
                ContentBlock::ToolUse { id, name, input }
            }
            "tool_result" => {
                let tool_use_id = block_obj
                    .get("tool_use_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::MessageParse("Missing 'tool_use_id' field".to_string()))?
                    .to_string();
                let content = block_obj.get("content").and_then(|v| {
                    serde_json::from_value(v.clone()).ok()
                });
                let is_error = block_obj.get("is_error").and_then(|v| v.as_bool());
                ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                }
            }
            _ => {
                return Err(Error::MessageParse(format!(
                    "Unknown content block type: {}",
                    block_type
                )))
            }
        };

        result.push(content_block);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_message() {
        let json = serde_json::json!({
            "type": "user",
            "message": {
                "content": "Hello"
            }
        });

        let message = parse_message(json).unwrap();
        match message {
            Message::User(user_msg) => {
                match user_msg.content {
                    MessageContent::String(s) => assert_eq!(s, "Hello"),
                    _ => panic!("Expected string content"),
                }
            }
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_parse_assistant_message() {
        let json = serde_json::json!({
            "type": "assistant",
            "message": {
                "model": "claude-3-sonnet",
                "content": [
                    {
                        "type": "text",
                        "text": "Hello!"
                    }
                ]
            }
        });

        let message = parse_message(json).unwrap();
        match message {
            Message::Assistant(assistant_msg) => {
                assert_eq!(assistant_msg.model, "claude-3-sonnet");
                assert_eq!(assistant_msg.content.len(), 1);
            }
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_parse_result_message() {
        let json = serde_json::json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 1000,
            "duration_api_ms": 800,
            "is_error": false,
            "num_turns": 1,
            "session_id": "test-123"
        });

        let message = parse_message(json).unwrap();
        match message {
            Message::Result(result_msg) => {
                assert_eq!(result_msg.subtype, "success");
                assert_eq!(result_msg.session_id, "test-123");
            }
            _ => panic!("Expected result message"),
        }
    }
}
