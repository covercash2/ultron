use std::{borrow::Cow, sync::Arc};

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters}, model::{ServerCapabilities, ServerInfo, *}, schemars, tool, tool_handler, tool_router, transport::{
        streamable_http_server::session::local::LocalSessionManager, StreamableHttpService
    }, ServerHandler
};
use tyche::dice::roller::FastRand;

use crate::{
    User,
    dice::RollerImpl,
    nlp::{ChatAgent, LmChatAgent},
};
use crate::{
    dice::{DiceRoll, DiceRollError},
    event_processor::EventProcessor,
};

pub mod client;

#[derive(Debug, Clone)]
pub struct UltronMcp<TRoller, TAgent: ChatAgent = LmChatAgent> {
    pub event_processor: Arc<EventProcessor<TRoller, TAgent>>,
}

pub struct UltronCommands<TRoller = FastRand, TAgent = LmChatAgent> {
    event_processor: Arc<EventProcessor<TRoller, TAgent>>,
    tool_router: ToolRouter<Self>,
}

impl<TRoller, TAgent> From<UltronMcp<TRoller, TAgent>>
    for StreamableHttpService<UltronCommands<TRoller, TAgent>>
where
    TRoller: RollerImpl + 'static,
    TAgent: ChatAgent + 'static,
{
    fn from(UltronMcp { event_processor }: UltronMcp<TRoller, TAgent>) -> Self {
        build(event_processor)
    }
}

pub fn build<TRoller, TAgent>(
    event_processor: Arc<EventProcessor<TRoller, TAgent>>,
) -> StreamableHttpService<UltronCommands<TRoller, TAgent>>
where
    TRoller: RollerImpl + 'static,
    TAgent: ChatAgent + 'static,
{
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
impl<TRoller, TAgent> UltronCommands<TRoller, TAgent>
where
    TRoller: RollerImpl + 'static,
    TAgent: ChatAgent + 'static,
{
    pub fn new(event_processor: Arc<EventProcessor<TRoller, TAgent>>) -> Self {
        Self {
            event_processor,
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
            .event_processor
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
impl<TRoller, TAgent> ServerHandler for UltronCommands<TRoller, TAgent>
where
    TRoller: RollerImpl + 'static,
    TAgent: ChatAgent + 'static,
{
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Commands available to query and control Ultron".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
