//! this module implements the HTTP server for Ultron
//! to accept commands from
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

use crate::{
    Channel, ChatBot,
    event_processor::{BotMessage, Event, EventError, EventProcessor, EventType},
};

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
        healthcheck,
        index,
        api_doc
    ),
    tags(
        (name = OpenApiTag::BotCommand.as_str(), description = "orders to submit to Ultron"),
        (name = OpenApiTag::Telemetry.as_str(), description = "figure out what's wrong with Ultron"),
        (name = OpenApiTag::Meta.as_str(), description = "meta information about Ultron"),
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
    #[strum(to_string = "/api_doc")]
    ApiDoc,
    #[strum(to_string = "/events")]
    Events,
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
        .routes(routes!(index))
        .routes(routes!(command, healthcheck))
        .routes(routes!(api_doc))
        .routes(routes!(events))
        .layer(TracingMiddleware::builder().build().make_layer())
        .with_state(state)
        .split_for_parts();

    router
}

#[utoipa::path(
    get,
    path = Route::ApiDoc.to_string(),
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
    user: String,
    /// command input as if it was a message from Discord,
    /// e.g. `echo hello`
    event_input: String,
    /// the type of event, e.g. `Command` or `NaturalLanguage`
    event_type: EventType,
}

impl From<BotInput> for Event {
    fn from(input: BotInput) -> Self {
        Self {
            user: input.user.into(),
            content: BotMessage::raw(input.event_input),
            event_type: input.event_type,
        }
    }
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
    let chat_input = Event::from(bot_input.clone());

    tracing::info!("response: {:?}", chat_input);

    match state.event_processor.process(chat_input).await? {
        Some(crate::Response::PlainChat(response)) => {
            state
                .chat_bot
                .send_message(bot_input.channel, &response)
                .await
                .map_err(|e| ServerError::ChatBot(Box::new(e)))?;
        }
        Some(crate::Response::Bot(bot_message)) => state
            .chat_bot
            .send_message(
                bot_input.channel,
                &bot_message.render_without_thinking_parts(),
            )
            .await
            .map_err(|e| ServerError::ChatBot(Box::new(e)))?,
        None => tracing::debug!("no response from event processor"),
    }

    Ok(())
}

/// tests bot input.
#[utoipa::path(
    get,
    path = Route::Events.to_string(),
    responses(
        (status = OK, description = "command sent"),
        (status = INTERNAL_SERVER_ERROR, description = "error sending message to Discord")
    ),
    tag = OpenApiTag::BotCommand.as_str(),
)]
async fn events<Bot>(State(state): State<AppState<Bot>>) -> Json<Vec<Event>> {
    let events: Vec<Event> = state.event_processor.dump_events().await;

    Json(events)
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
            event_processor: Arc::new(EventProcessor::default()),
            chat_bot: Arc::new(TestBot),
        };
        let bot_input = BotInput {
            channel: Channel::Debug,
            user: "anonymous".to_string(),
            event_input: "echo hello".to_string(),
            event_type: EventType::Command,
        };
        let json = Json(bot_input);
        let () = command(State(state), json)
            .await
            .expect("got an error from the test bot");
    }
}
