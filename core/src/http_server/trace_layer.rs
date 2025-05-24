use bon::{builder, Builder};
use tower_http::{
    classify::{ServerErrorsAsFailures, SharedClassifier},
    trace::{
        DefaultMakeSpan, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
        TraceLayer,
    },
};
use tracing::Level;

#[derive(Clone, Debug, Builder)]
pub struct TracingMiddleware {
    #[builder(default = Level::INFO)]
    level: Level,
    #[builder(default = true)]
    include_headers: bool,
}

impl TracingMiddleware {
    pub fn make_layer(self) -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
        TraceLayer::new_for_http()
            .make_span_with(
                DefaultMakeSpan::new()
                    .level(self.level)
                    .include_headers(self.include_headers),
            )
            .on_request(DefaultOnRequest::new().level(self.level))
            .on_response(
                DefaultOnResponse::new()
                    .level(self.level)
                    .include_headers(self.include_headers),
            )
            .on_failure(DefaultOnFailure::new().level(self.level))
            .on_eos(DefaultOnEos::new().level(self.level))
    }
}

