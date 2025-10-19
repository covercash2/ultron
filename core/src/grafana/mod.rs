use axum::extract::{Json as JsonPayload, State};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::{
    Channel,
    chatbot::ChatBot,
    http_server::{AppState, HttpRoute, OpenApiTag},
};

#[cfg(test)]
mod testdata;

pub struct GrafanaRoute;

impl HttpRoute for GrafanaRoute {
    const PATH: &'static str = "/grafana";
}

#[utoipa::path(
    post,
    path = GrafanaRoute::PATH,
    responses(
        (status = OK, description = "webhook received")
    ),
    tag = OpenApiTag::Telemetry.as_str(),
)]
pub async fn webhook_handler<TBot>(
    State(state): State<AppState<TBot>>,
    JsonPayload(payload): JsonPayload<GrafanaAlertPayload>,
) -> Result<(), &'static str>
where
    TBot: ChatBot + 'static,
{
    tracing::debug!("received grafana webhook: {:?}", payload);

    let title = &payload.title;
    let message = &payload.message;

    let response = format!("# 🚨🚨🚨🚨 {title} 🚨🚨🚨🚨\n\n{message}");

    state
        .chat_bot
        .send_message(Channel::Debug, &response)
        .await
        .map_err(|_| "failed to send message")?;

    Ok(())
}

/// represents a single alert in a Grafana alert payload.
/// this is generated from a webhook:
/// <https://grafana.com/docs/grafana/latest/alerting/configure-notifications/manage-contact-points/integrations/webhook-notifier/#body>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrafanaAlertPayload {
    /// <https://grafana.com/docs/grafana/latest/alerting/configure-notifications/manage-contact-points/integrations/webhook-notifier/#optional-settings-using-templates>
    pub title: String,
    pub message: String,
    /// Name of the contact point.
    pub receiver: String,
    /// Current status of the alert, firing or resolved.
    pub status: String,
    /// ID of the organization related to the payload.
    pub org_id: u32,
    /// Alerts that are triggering.
    pub alerts: Vec<GrafanaAlert>,
    /// Labels that are used for grouping, map of string keys to string values.
    pub group_labels: Json,
    /// Labels that all alarms have in common, map of string keys to string values.
    pub common_labels: Json,
    /// Annotations that all alarms have in common, map of string keys to string values.
    pub common_annotations: Json,
    /// External URL to the Grafana instance sending this webhook.
    #[serde(rename = "externalURL")]
    pub external_url: String,
    /// Version of the payload structure.
    pub version: String,
    /// Key that is used for grouping.
    pub group_key: String,
    /// Number of alerts that were truncated.
    pub truncated_alerts: u32,
    /// State of the alert group (either alerting or ok).
    pub state: String,
}

/// represents a single alert in a Grafana alert payload.
/// this is generated from a webhook:
/// <https://grafana.com/docs/grafana/latest/alerting/configure-notifications/manage-contact-points/integrations/webhook-notifier/#alert>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrafanaAlert {
    /// current status of the alert, firing or resolved
    pub status: String,
    /// labels that are part of this alert, map of string keys to string values
    pub labels: Json,
    /// annotations that are part of this alert, map of string keys to string values
    pub annotations: Json,
    /// start time of the alert
    pub starts_at: String,
    /// end time of the alert, default value when not resolved is 0001-01-01T00:00:00Z
    pub ends_at: String,
    /// values that triggered the current status
    pub values: Json,
    /// URL of the alert rule in the Grafana UI
    #[serde(rename = "generatorURL")]
    pub generator_url: String,
    /// the labels fingerprint, alarms with the same labels will have the same fingerprint
    pub fingerprint: String,
    /// URL to silence the alert rule in the Grafana UI
    #[serde(rename = "silenceURL")]
    pub silence_url: String,
    /// a link to the Grafana Dashboard if the alert has a Dashboard UID annotation
    #[serde(rename = "dashboardURL")]
    pub dashboard_url: Option<String>,
    /// a link to the panel if the alert has a Panel ID annotation
    #[serde(rename = "panelURL")]
    pub panel_url: Option<String>,
    /// URL of a screenshot of a panel assigned to the rule that created this notification
    #[serde(rename = "imageURL")]
    pub image_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grafana_alert_payload_deserialize() {
        let example: GrafanaAlertPayload = serde_json::from_str(testdata::GRAFANA_EXAMPLE)
            .expect("unable to deserialize grafana example");

        let string =
            serde_json::to_string_pretty(&example).expect("unable to serialize grafana example");

        let example_roundtrip: GrafanaAlertPayload =
            serde_json::from_str(&string).expect("unable to deserialize grafana example");

        assert_eq!(example, example_roundtrip);
    }
}
