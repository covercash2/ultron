use std::{borrow::Cow, sync::Arc};

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, *},
    schemars, tool, tool_handler, tool_router,
    transport::{
        StreamableHttpService, streamable_http_server::session::local::LocalSessionManager,
    },
};

use crate::User;
use crate::{
    dice::{DiceRoll, DiceRollError},
    event_processor::EventProcessor,
};

pub mod client;

#[derive(Debug, Clone)]
pub struct UltronMcp {
    pub event_processor: Arc<EventProcessor>,
}

pub struct UltronCommands {
    event_processor: Arc<EventProcessor>,
    // TODO: parameterize the RNG
    dice_roller: crate::dice::DiceRoller<tyche::dice::roller::FastRand>,
    tool_router: ToolRouter<Self>,
}

impl From<UltronMcp> for StreamableHttpService<UltronCommands> {
    fn from(UltronMcp { event_processor }: UltronMcp) -> Self {
        build(event_processor)
    }
}

pub fn build(event_processor: Arc<EventProcessor>) -> StreamableHttpService<UltronCommands> {
    let event_processor = event_processor.clone();
    StreamableHttpService::new(
        move || Ok(UltronCommands::new(event_processor.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    )
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DiceRollRequest {
    #[schemars(description = "a dice expression according to the tyche library and Foundry VTT")]
    #[schemars(example = "4d6kh3+2")]
    #[schemars(example = "1d20+5")]
    #[schemars(example = "2d10")]
    #[schemars(example = "4d6+2d8-2")]
    expression: String,
}

#[tool_router]
impl UltronCommands {
    pub fn new(event_processor: Arc<EventProcessor>) -> Self {
        Self {
            event_processor,
            dice_roller: crate::dice::DiceRoller::default(),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "get the system prompt for Ultron")]
    pub async fn system_prompt(&self) -> String {
        self.event_processor
            .dump_events()
            .await
            .into_iter()
            .filter_map(|event| {
                let User::System = event.user else {
                    return None;
                };

                Some(event.content.render_without_thinking_parts())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[tool(description = "roll dice given Foundry VTT/tyche expression")]
    pub async fn roll_dice(
        &self,
        Parameters(DiceRollRequest { expression }): Parameters<DiceRollRequest>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        tracing::debug!(expression, "rolling dice");

        let dice_roll: DiceRoll = self
            .dice_roller
            .clone()
            .roll_string(&expression)
            .and_then(|evaled| evaled.try_into())
            .map_err(ErrorData::from)?;

        Ok(CallToolResult::success(vec![Content::json(dice_roll)?]))
    }
}

impl From<DiceRollError> for rmcp::ErrorData {
    /// https://www.jsonrpc.org/specification
    /// these error codes are part of the response to the agent,
    /// so they need to be more semantically correct
    /// than technically correct
    fn from(error: DiceRollError) -> Self {
        let code = match error {
            DiceRollError::Parse(_) => ErrorCode::INVALID_REQUEST,
            DiceRollError::Eval(_) => ErrorCode::INVALID_PARAMS,
            DiceRollError::Calc(_) => ErrorCode::INVALID_PARAMS,
        };

        let message: Cow<'_, str> = error.to_string().into();

        rmcp::ErrorData {
            code,
            message,
            data: None,
        }
    }
}

#[tool_handler]
impl ServerHandler for UltronCommands {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Commands available to query and control Ultron".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
