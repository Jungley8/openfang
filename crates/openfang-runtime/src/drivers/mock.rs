//! Mock LLM driver for benchmarks.
//!
//! Returns immediately with a fixed text response so that send_message → execute_llm_agent → run_agent_loop
//! can be measured without network or real model.

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError, StreamEvent};
use async_trait::async_trait;
use openfang_types::message::{ContentBlock, StopReason, TokenUsage};
use std::sync::Arc;

/// Mock driver that completes immediately with a fixed response.
#[derive(Default)]
pub struct MockLlmDriver;

impl MockLlmDriver {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl LlmDriver for MockLlmDriver {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            content: vec![ContentBlock::Text {
                text: "OK".to_string(),
            }],
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 2,
            },
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<CompletionResponse, LlmError> {
        let r = self.complete(request).await?;
        let _ = tx
            .send(StreamEvent::TextDelta {
                text: "OK".to_string(),
            })
            .await;
        let _ = tx
            .send(StreamEvent::ContentComplete {
                stop_reason: r.stop_reason,
                usage: r.usage,
            })
            .await;
        Ok(r)
    }
}
