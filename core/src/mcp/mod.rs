use std::sync::Arc;

use rmcp::{
    RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    model::{ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, tool_handler, tool_router,
};

use crate::User;
use crate::event_processor::EventProcessor;

struct UltronCommands {
    event_processor: Arc<EventProcessor>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl UltronCommands {
    pub fn new(event_processor: Arc<EventProcessor>) -> Self {
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

    // #[tool(description = "roll dice given the dice number and values")]
    // pub async fn roll_dice(&self, parameters: Parameters) -> String {
    //
    // }
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
