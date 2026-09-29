use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatSnapshot {
    pub revision: u64,
    pub connected: bool,
    pub busy: bool,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct ChatMessage {
    pub id: String,
    pub role: Role,
    pub parts: Vec<MessagePart>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Role {
    User,
    Assistant,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ToolState {
    Pending,
    Running,
    Failed,
    Complete,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum MessagePart {
    Text {
        text: String,
        html: String,
    },
    Thinking {
        text: String,
    },
    Tool {
        id: String,
        name: String,
        arguments: String,
        output: String,
        state: ToolState,
    },
}

#[derive(Deserialize)]
struct Message {
    role: String,
    content: Content,
    #[serde(rename = "stopReason")]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    Blocks(Vec<Block>),
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Block {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "toolCall")]
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct ToolResult {
    content: Content,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum Event {
    AgentStart,
    AgentSettled,
    MessageStart {
        message: Message,
    },
    MessageUpdate {
        assistant_message_event: MessageUpdate,
    },
    MessageEnd {
        message: Message,
    },
    ToolExecutionStart {
        tool_call_id: String,
    },
    ToolExecutionUpdate {
        tool_call_id: String,
        partial_result: ToolResult,
    },
    ToolExecutionEnd {
        tool_call_id: String,
        result: ToolResult,
        is_error: bool,
    },
    AutoRetryStart,
    AutoRetryEnd {
        success: bool,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum MessageUpdate {
    TextDelta {
        content_index: usize,
        delta: String,
    },
    ThinkingDelta {
        content_index: usize,
        delta: String,
    },
    TextEnd {
        content_index: usize,
        content: String,
    },
    ThinkingEnd {
        content_index: usize,
        content: String,
    },
    #[serde(other)]
    Other,
}

fn content_text(content: Content) -> String {
    match content {
        Content::Text(text) => text,
        Content::Blocks(blocks) => blocks
            .into_iter()
            .filter_map(|block| match block {
                Block::Text { text } => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn message_parts(content: Content) -> Vec<MessagePart> {
    let blocks = match content {
        Content::Text(text) => vec![Block::Text { text }],
        Content::Blocks(blocks) => blocks,
    };
    blocks
        .into_iter()
        .filter_map(|block| match block {
            Block::Text { text } => Some(MessagePart::Text {
                html: cms_core::markdown::render(&text),
                text,
            }),
            Block::Thinking { thinking } => Some(MessagePart::Thinking { text: thinking }),
            Block::ToolCall {
                id,
                name,
                arguments,
            } => Some(MessagePart::Tool {
                id,
                name,
                arguments: serde_json::to_string_pretty(&arguments)
                    .expect("JSON value is serializable"),
                output: String::new(),
                state: ToolState::Pending,
            }),
            Block::Other => None,
        })
        .collect()
}

impl ChatSnapshot {
    pub fn apply(&mut self, record: &Value) -> Result<(), serde_json::Error> {
        let event = Event::deserialize(record)?;
        match event {
            Event::AgentStart => {
                self.busy = true;
                self.error = None;
            }
            Event::AgentSettled => self.busy = false,
            Event::MessageStart { message } => {
                let role = match message.role.as_str() {
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    _ => return Ok(()),
                };
                self.messages.push(ChatMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    role,
                    parts: message_parts(message.content),
                });
            }
            Event::MessageUpdate {
                assistant_message_event,
            } => {
                let (index, thinking, content, replace) = match assistant_message_event {
                    MessageUpdate::TextDelta {
                        content_index,
                        delta,
                    } => (content_index, false, delta, false),
                    MessageUpdate::ThinkingDelta {
                        content_index,
                        delta,
                    } => (content_index, true, delta, false),
                    MessageUpdate::TextEnd {
                        content_index,
                        content,
                    } => (content_index, false, content, true),
                    MessageUpdate::ThinkingEnd {
                        content_index,
                        content,
                    } => (content_index, true, content, true),
                    MessageUpdate::Other => return Ok(()),
                };
                if index >= 4096 {
                    return Ok(());
                }
                let Some(message) = self
                    .messages
                    .last_mut()
                    .filter(|message| matches!(message.role, Role::Assistant))
                else {
                    return Ok(());
                };
                while message.parts.len() <= index {
                    message.parts.push(MessagePart::Text {
                        text: String::new(),
                        html: String::new(),
                    });
                }
                let part = &mut message.parts[index];
                if thinking && !matches!(part, MessagePart::Thinking { .. }) {
                    *part = MessagePart::Thinking {
                        text: String::new(),
                    };
                }
                if let MessagePart::Text { text, .. } | MessagePart::Thinking { text } = part {
                    if replace {
                        *text = content;
                    } else {
                        text.push_str(&content);
                    }
                }
            }
            Event::MessageEnd { message: completed } if completed.role == "assistant" => {
                if let Some(message) = self
                    .messages
                    .last_mut()
                    .filter(|message| matches!(message.role, Role::Assistant))
                {
                    message.parts = message_parts(completed.content);
                }
                if completed.stop_reason.as_deref() == Some("error") {
                    self.error = Some("The model request failed. Check Pi authentication and provider availability.".into());
                }
            }
            event @ (Event::ToolExecutionStart { .. }
            | Event::ToolExecutionUpdate { .. }
            | Event::ToolExecutionEnd { .. }) => {
                let (call_id, result, next_state) = match event {
                    Event::ToolExecutionStart { tool_call_id } => {
                        (tool_call_id, None, Some(ToolState::Running))
                    }
                    Event::ToolExecutionUpdate {
                        tool_call_id,
                        partial_result,
                    } => (tool_call_id, Some(partial_result), None),
                    Event::ToolExecutionEnd {
                        tool_call_id,
                        result,
                        is_error,
                    } => (
                        tool_call_id,
                        Some(result),
                        Some(if is_error {
                            ToolState::Failed
                        } else {
                            ToolState::Complete
                        }),
                    ),
                    _ => unreachable!("matched tool execution event"),
                };
                let output = result.map(|result| content_text(result.content));
                for message in &mut self.messages {
                    for part in &mut message.parts {
                        if let MessagePart::Tool {
                            id,
                            output: current_output,
                            state,
                            ..
                        } = part
                        {
                            if *id != call_id {
                                continue;
                            }
                            if let Some(output) = &output {
                                current_output.clone_from(output);
                            }
                            if let Some(next_state) = &next_state {
                                *state = next_state.clone();
                            }
                        }
                    }
                }
            }
            Event::AutoRetryStart => self.error = Some("Pi is retrying the model request…".into()),
            Event::AutoRetryEnd { success: true } => self.error = None,
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reconciles_message_blocks() {
        let mut state = ChatSnapshot::default();
        state.apply(&json!({"type":"agent_start"})).unwrap();
        state
            .apply(&json!({"type":"message_start","message":{"role":"assistant","content":[]}}))
            .unwrap();
        for (index, kind, delta) in [
            (0, "thinking_delta", "why"),
            (1, "text_delta", "hello"),
            (1, "text_delta", " world"),
        ] {
            state.apply(&json!({"type":"message_update","assistantMessageEvent":{"type":kind,"contentIndex":index,"delta":delta}})).unwrap();
        }
        assert!(
            matches!(&state.messages[0].parts[1], MessagePart::Text {text,..} if text == "hello world")
        );
        state.apply(&json!({"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"final"}]}})).unwrap();
        assert!(
            matches!(&state.messages[0].parts[0], MessagePart::Text {text,..} if text == "final")
        );
        state.apply(&json!({"type":"agent_end"})).unwrap();
        assert!(state.busy);
        state.apply(&json!({"type":"agent_settled"})).unwrap();
        assert!(!state.busy);
    }

    #[test]
    fn projects_tools_and_redacts_errors() {
        let mut state = ChatSnapshot::default();
        state
            .apply(&json!({"type":"message_start","message":{"role":"assistant","content":[]}}))
            .unwrap();
        state.apply(&json!({"type":"message_end","message":{"role":"assistant","content":[{"type":"toolCall","id":"a","name":"read","arguments":{"path":"x"}}]}})).unwrap();
        state
            .apply(&json!({"type":"tool_execution_start","toolCallId":"a"}))
            .unwrap();
        state.apply(&json!({"type":"tool_execution_end","toolCallId":"a","result":{"content":[{"type":"text","text":"result"}]},"isError":false})).unwrap();
        assert!(
            matches!(&state.messages[0].parts[0], MessagePart::Tool {output,state,..} if output == "result" && matches!(state, ToolState::Complete))
        );
        state.apply(&json!({"type":"message_end","message":{"role":"assistant","content":[],"stopReason":"error","errorMessage":"secret credential"}})).unwrap();
        assert!(!state.error.unwrap().contains("secret"));
    }
    #[test]
    fn rejects_malformed_events() {
        let mut snapshot = ChatSnapshot::default();
        assert!(snapshot.apply(&json!({"type":"message_start","message":{"role":"assistant","content":[{"type":"text"}]}})).is_err());
        snapshot.apply(&json!({"type":"future_event"})).unwrap();
        assert!(snapshot.messages.is_empty());
    }
    #[test]
    fn completes_streamed_blocks() {
        let mut snapshot = ChatSnapshot::default();
        snapshot
            .apply(&json!({"type":"message_start","message":{"role":"user","content":"question"}}))
            .unwrap();
        snapshot
            .apply(&json!({"type":"message_start","message":{"role":"assistant","content":[]}}))
            .unwrap();
        for (kind, index, content) in [("thinking_end", 0, "reason"), ("text_end", 1, "answer")] {
            snapshot.apply(&json!({"type":"message_update","assistantMessageEvent":{"type":kind,"contentIndex":index,"content":content}})).unwrap();
        }
        snapshot.apply(&json!({"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":4096,"delta":"ignore"}})).unwrap();
        let reply = &snapshot.messages[1];
        assert_eq!(reply.parts.len(), 2);
        assert!(matches!(&reply.parts[0], MessagePart::Thinking { text } if text == "reason"));
        assert!(matches!(&reply.parts[1], MessagePart::Text { text, .. } if text == "answer"));
        snapshot.apply(&json!({"type":"auto_retry_start"})).unwrap();
        assert!(snapshot.error.is_some());
        snapshot
            .apply(&json!({"type":"auto_retry_end","success":true}))
            .unwrap();
        assert!(snapshot.error.is_none());
    }

    #[test]
    fn projects_tool_progress_and_failure() {
        let mut snapshot = ChatSnapshot::default();
        snapshot.apply(&json!({"type":"message_start","message":{"role":"assistant","content":[{"type":"toolCall","id":"read","name":"read","arguments":{"path":"a"}}]}})).unwrap();
        snapshot
            .apply(&json!({"type":"tool_execution_start","toolCallId":"read"}))
            .unwrap();
        snapshot.apply(&json!({"type":"tool_execution_update","toolCallId":"read","partialResult":{"content":[{"type":"text","text":"partial"},{"type":"image","data":"ignored"}]}})).unwrap();
        assert!(
            matches!(&snapshot.messages[0].parts[0], MessagePart::Tool { output, state, .. } if output == "partial" && matches!(state, ToolState::Running))
        );
        snapshot.apply(&json!({"type":"tool_execution_end","toolCallId":"read","result":{"content":"failed to read"},"isError":true})).unwrap();
        assert!(
            matches!(&snapshot.messages[0].parts[0], MessagePart::Tool { output, state, .. } if output == "failed to read" && matches!(state, ToolState::Failed))
        );
    }
}
