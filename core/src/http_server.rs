use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bon::Builder;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use trace_layer::TracingMiddleware;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{ApiInput, Channel, ChatBot, EventError, EventProcessor};

mod trace_layer;

#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, strum::Display, strum::IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
pub enum OpenApiTag {
    BotCommand,
    Telemetry,
    Meta,
}

impl OpenApiTag {
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

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

    #[error("failed to generate OpenAPI doc")]
    OpenApiDocGeneration,
}

#[derive(Builder, Debug, Clone)]
pub struct AppState<ChatBot> {
    pub event_processor: Arc<EventProcessor>,
    pub chat_bot: Arc<ChatBot>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        command,
        echo,
        healthcheck,
        index,
        api_doc
    ),
    tags(
        (name = OpenApiTag::BotCommand.as_str(), description = "orders to submit to Ultron"),
        (name = OpenApiTag::Telemetry.as_str(), description = "figure out what's wrong with Ultron")
    )
)]
struct ApiDoc;

/// Routes for the HTTP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, strum::Display, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Route {
    #[strum(to_string = "/command")]
    Command,
    #[strum(to_string = "/echo")]
    Echo,
    #[strum(to_string = "/healthcheck")]
    Healthcheck,
    #[strum(to_string = "/")]
    Index,
}

impl Route {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

/// Starts the HTTP server on the specified port with the given application state.
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

    let (router, _api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(echo, index))
        .routes(routes!(command, healthcheck))
        .routes(routes!(api_doc))
        .layer(TracingMiddleware::builder().build().make_layer())
        .with_state(state)
        .split_for_parts();

    router
}

#[utoipa::path(
    get,
    path = Route::Index.to_string(),
    responses(
        (status = OK, description = "index page")
    ),
    tag = OpenApiTag::Meta.as_str(),
)]
async fn api_doc() -> ServerResult<Json<String>> {
    ApiDoc::openapi()
        .to_json()
        .map_err(|_| ServerError::OpenApiDocGeneration)
        .map(Json)
}

/// index route: [`Route::Index`]
#[utoipa::path(
    get,
    path = Route::Index.to_string(),
    responses(
        (status = OK, description = "index page")
    ),
    tag = OpenApiTag::Telemetry.as_str(),
)]
async fn index() -> String {
    "Hello, World!".into()
}

#[utoipa::path(
    get,
    path = Route::Healthcheck.to_string(),
    responses(
        (status = OK, description = "healthcheck OK")
    ),
    tag = OpenApiTag::Telemetry.as_str(),
)]
async fn healthcheck() -> String {
    "OK".into()
}

/// input to the bot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BotInput {
    /// the channel to send the command to
    channel: Channel,
    /// command input as if it was a message from Discord,
    /// e.g. `echo hello`
    command_input: String,
}

/// tests bot input.
#[utoipa::path(
    post,
    path = Route::Command.to_string(),
    responses(
        (status = OK, description = "command sent"),
        (status = INTERNAL_SERVER_ERROR, description = "error sending message to Discord")
    ),
    tag = OpenApiTag::BotCommand.as_str(),
)]
async fn command<Bot>(
    State(state): State<AppState<Bot>>,
    Json(bot_input): Json<BotInput>,
) -> Result<(), ServerError>
where
    Bot: ChatBot + 'static,
{
    let chat_input = ApiInput::from(bot_input.command_input);

    tracing::info!("response: {:?}", chat_input);

    match state.event_processor.process(chat_input).await? {
        crate::Response::PlainChat(response) => {
            state
                .chat_bot
                .send_message(bot_input.channel, &response)
                .await
                .map_err(|e| ServerError::ChatBot(Box::new(e)))?;
        }
    }

    Ok(())
}

/// make Ultron say something
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EchoInput {
    /// the channel to send the command to
    channel: Channel,
    /// what Ultron is going to say
    message: String,
}

impl From<EchoInput> for BotInput {
    fn from(input: EchoInput) -> Self {
        Self {
            channel: input.channel,
            command_input: format!("echo {}", input.message),
        }
    }
}

/// Make Ultron say something
#[utoipa::path(
    post,
    path = Route::Echo.to_string(),
    responses(
        (status = OK, description = "echo command sent"),
        (status = INTERNAL_SERVER_ERROR, description = "error sending message to Discord")
    ),
    tag = OpenApiTag::BotCommand.as_str(),
)]
async fn echo<Bot>(
    State(state): State<AppState<Bot>>,
    Json(input): Json<EchoInput>,
) -> Result<(), ServerError>
where
    Bot: ChatBot + 'static,
{
    let input: BotInput = input.into();
    let api_input = ApiInput::from(input.command_input);

    match state.event_processor.process(api_input).await? {
        crate::Response::PlainChat(response) => {
            state
                .chat_bot
                .send_message(input.channel, &response)
                .await
                .map_err(|e| ServerError::ChatBot(Box::new(e)))?;
        }
    }

    Ok(())
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        tracing::warn!("error: {}", self);
        let status = match self {
            ServerError::UnableToBindPort(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Startup(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Event(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::ChatBot(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::OpenApiDocGeneration => StatusCode::INTERNAL_SERVER_ERROR,
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
            command_input: "echo hello".to_string(),
        };
        let json = Json(bot_input);
        let () = command(State(state), json)
            .await
            .expect("got an error from the test bot");
    }
}
