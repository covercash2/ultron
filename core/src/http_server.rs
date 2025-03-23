use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bon::Builder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ApiInput, Channel, ChatBot, ChatInput, EventError, EventProcessor};

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("unable to bind to port {0}")]
    UnableToBindPort(u16),
    #[error("error running server: {0}")]
    Startup(std::io::Error),
    #[error("error processing event: {0}")]
    Event(#[from] EventError),
    #[error("error invoking chat bot: {0}")]
    ChatBot(Box<dyn std::error::Error>),
}

#[derive(Builder, Debug, Clone)]
pub struct AppState<ChatBot> {
    pub event_processor: Arc<EventProcessor>,
    pub chat_bot: Arc<ChatBot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Route {
    #[strum(to_string = "/")]
    Index,
    Bot,
}

pub async fn serve<Bot>(port: u16, state: AppState<Bot>) -> ServerResult<()>
where
    Bot: ChatBot + 'static,
{
    let router = create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .map_err(|_| ServerError::UnableToBindPort(port))?;
    axum::serve(listener, router)
        .await
        .map_err(ServerError::Startup)?;

    Ok(())
}

pub fn create_router<Bot>(state: AppState<Bot>) -> Router
where
    Bot: ChatBot + 'static,
{
    Router::new()
        .route("/", get(index))
        .route("/bot/say", post(bot))
        .with_state(state)
}

/// index route: [`Route::Index`]
async fn index() -> String {
    "Hello, World!".into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotInput {
    channel: Channel,
    message: String,
}

async fn bot<Bot>(
    State(state): State<AppState<Bot>>,
    Json(bot_input): Json<BotInput>,
) -> Result<(), ServerError>
where
    Bot: ChatBot + 'static,
{
    let chat_input = ApiInput::from(bot_input.message);

    tracing::info!("response: {:?}", chat_input);

    match state.event_processor.process(chat_input).await? {
        Some(crate::Response::PlainChat(response)) => {
            state
                .chat_bot
                .send_message(bot_input.channel, &response)
                .await
                .map_err(|e| ServerError::ChatBot(Box::new(e)))?;
        }
        None => {}
    }

    Ok(())
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = match self {
            ServerError::UnableToBindPort(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Startup(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Event(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::ChatBot(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestBot;

    impl ChatBot for TestBot {
        type Error = std::io::Error;

        async fn send_message(&self, _channel: Channel, _message: &str) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_index() {
        let response = index().await;
        assert_eq!(response, "Hello, World!".to_string());
    }

    #[tokio::test]
    async fn test_bot() {
        let state = AppState {
            event_processor: Arc::new(EventProcessor),
            chat_bot: Arc::new(TestBot),
        };
        let bot_input = BotInput {
            channel: Channel::Debug,
            message: "echo hello".to_string(),
        };
        let json = Json(bot_input);
        let () = bot(State(state), json)
            .await
            .expect("got an error from the test bot");
    }
}
