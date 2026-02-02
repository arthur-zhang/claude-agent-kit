//! Message parser for Claude Code SDK responses.

use crate::types::{
    AssistantMessage, ContentBlock, Error, Message, MessageContent, ProtocolMessage, Result,
    ResultMessage, StreamEvent, SystemMessage, UserMessage,
};



/// Convert ProtocolMessage to Message type.
///
/// This function provides a direct conversion from the strongly-typed ProtocolMessage
/// (used internally for protocol handling) to the user-facing Message type.
///
/// # Arguments
/// * `protocol_msg` - Strongly-typed protocol message
///
/// # Returns
/// Converted Message object
///
/// # Errors
/// Returns `Error::MessageParse` if the protocol message cannot be converted
pub fn protocol_message_to_message(protocol_msg: ProtocolMessage) -> Result<Message> {
    match protocol_msg {
        ProtocolMessage::User { message, parent_tool_use_id, uuid, .. } => {
            Ok(Message::User(UserMessage {
                content: message.content,
                uuid,
                parent_tool_use_id,
            }))
        }
        ProtocolMessage::Assistant { message, parent_tool_use_id, .. } => {
            Ok(Message::Assistant(AssistantMessage {
                content: message.content,
                model: message.model,
                parent_tool_use_id,
                error: None,
            }))
        }
        ProtocolMessage::Stream(msg) => Ok(Message::Stream(msg)),
        ProtocolMessage::Result(msg) => Ok(Message::Result(msg)),
        ProtocolMessage::System(msg) => Ok(Message::System(msg)),
        ProtocolMessage::ControlRequest { .. } => Err(Error::MessageParse(
            "ControlRequest should not be converted to Message".to_string(),
        )),
        ProtocolMessage::ControlResponse { .. } => Err(Error::MessageParse(
            "ControlResponse should not be converted to Message".to_string(),
        )),
    }
}
