use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use rmcp::{
    RoleClient, ServiceError, ServiceExt,
    model::{CallToolRequestParam, ClientInfo, InitializeRequestParam, Tool},
    service::{ClientInitializeError, RunningService},
    transport::StreamableHttpClientTransport,
};

pub type McpClientResult<T> = std::result::Result<T, ClientError>;
pub type InnerClient = RunningService<RoleClient, InitializeRequestParam>;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("failed to initialize client: {source}")]
    Init {
        #[from]
        source: ClientInitializeError,
    },

    #[error("failed to list tools: {0}")]
    ListTools(ServiceError),

    #[error("failed to call tool '{name}': {source}")]
    CallTool { name: String, source: ServiceError },
}

#[derive(Debug, Clone)]
pub struct McpClient {
    info: ClientInfo,
    inner: Arc<InnerClient>,
    tools: Arc<RwLock<ToolSet>>,
}

#[derive(Debug, Clone, PartialEq, schemars::JsonSchema)]
pub struct ToolSet(pub BTreeMap<Cow<'static, str>, Tool>);

impl FromIterator<(Cow<'static, str>, Tool)> for ToolSet {
    fn from_iter<T: IntoIterator<Item = (Cow<'static, str>, Tool)>>(iter: T) -> Self {
        ToolSet(iter.into_iter().collect())
    }
}

impl McpClient {
    pub async fn new(server_uri: &str) -> McpClientResult<Self> {
        // uses current package info for client info
        // let transport = StreamableHttpClientTransport::from(server_uri);
        let transport = StreamableHttpClientTransport::from_uri(server_uri);
        let client_info = ClientInfo::default();
        let client: InnerClient = client_info.clone().serve(transport).await?;

        let server_info = client.peer_info();

        tracing::info!("connected to server: {server_info:?}");

        let tools = list_tools(&client).await?;

        tracing::info!("available tools: {tools:#?}");

        let tools = Arc::new(RwLock::new(tools));

        Ok(McpClient {
            info: client_info,
            inner: client.into(),
            tools,
        })
    }

    pub async fn localhost(port: u16) -> McpClientResult<Self> {
        let server_uri = format!("http://localhost:{port}");
        Self::new(&server_uri).await
    }

    pub fn info(&self) -> &ClientInfo {
        &self.info
    }

    pub async fn call_tool(&self, tool_name: &str) -> McpClientResult<()> {
        self.inner
            .call_tool(CallToolRequestParam {
                name: tool_name.to_owned().into(),
                arguments: serde_json::json!({}).as_object().cloned(),
            })
            .await
            .map_err(|source| ClientError::CallTool {
                name: tool_name.into(),
                source,
            })?;

        Ok(())
    }

    pub async fn refresh_tools(&self) -> McpClientResult<ToolSet> {
        let tools = list_tools(&self.inner).await?;

        *self.tools.write().expect("tools lock poisoned") = tools;

        Ok(self
            .tools
            .read()
            .expect("tools lock poisoned")
            .clone())
    }
}

/// list tools, transforming upstream errors
async fn list_tools(client: &InnerClient) -> McpClientResult<ToolSet> {
    let tools = client
        .list_all_tools()
        .await
        .map_err(ClientError::ListTools)?
        .into_iter()
        .map(|tool| {
            let name = tool.name.clone();
            (name, tool)
        })
        .collect();

    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_toolset_schema() {
        let schema: schemars::Schema = schemars::schema_for!(ToolSet);
        insta::assert_json_snapshot!(schema, @r##"
        {
          "$schema": "https://json-schema.org/draft/2020-12/schema",
          "title": "ToolSet",
          "type": "object",
          "additionalProperties": {
            "$ref": "#/$defs/Tool"
          },
          "$defs": {
            "Tool": {
              "description": "A tool that can be used by a model.",
              "type": "object",
              "properties": {
                "name": {
                  "description": "The name of the tool",
                  "type": "string"
                },
                "title": {
                  "description": "A human-readable title for the tool",
                  "type": [
                    "string",
                    "null"
                  ]
                },
                "description": {
                  "description": "A description of what the tool does",
                  "type": [
                    "string",
                    "null"
                  ]
                },
                "inputSchema": {
                  "description": "A JSON Schema object defining the expected parameters for the tool",
                  "type": "object",
                  "additionalProperties": true
                },
                "outputSchema": {
                  "description": "An optional JSON Schema object defining the structure of the tool's output",
                  "type": [
                    "object",
                    "null"
                  ],
                  "additionalProperties": true
                },
                "annotations": {
                  "description": "Optional additional tool information.",
                  "anyOf": [
                    {
                      "$ref": "#/$defs/ToolAnnotations"
                    },
                    {
                      "type": "null"
                    }
                  ]
                },
                "icons": {
                  "description": "Optional list of icons for the tool",
                  "type": [
                    "array",
                    "null"
                  ],
                  "items": {
                    "$ref": "#/$defs/Icon"
                  }
                }
              },
              "required": [
                "name",
                "inputSchema"
              ]
            },
            "ToolAnnotations": {
              "description": "Additional properties describing a Tool to clients.\n\nNOTE: all properties in ToolAnnotations are **hints**.\nThey are not guaranteed to provide a faithful description of\ntool behavior (including descriptive properties like `title`).\n\nClients should never make tool use decisions based on ToolAnnotations\nreceived from untrusted servers.",
              "type": "object",
              "properties": {
                "title": {
                  "description": "A human-readable title for the tool.",
                  "type": [
                    "string",
                    "null"
                  ]
                },
                "readOnlyHint": {
                  "description": "If true, the tool does not modify its environment.\n\nDefault: false",
                  "type": [
                    "boolean",
                    "null"
                  ]
                },
                "destructiveHint": {
                  "description": "If true, the tool may perform destructive updates to its environment.\nIf false, the tool performs only additive updates.\n\n(This property is meaningful only when `readOnlyHint == false`)\n\nDefault: true\nA human-readable description of the tool's purpose.",
                  "type": [
                    "boolean",
                    "null"
                  ]
                },
                "idempotentHint": {
                  "description": "If true, calling the tool repeatedly with the same arguments\nwill have no additional effect on the its environment.\n\n(This property is meaningful only when `readOnlyHint == false`)\n\nDefault: false.",
                  "type": [
                    "boolean",
                    "null"
                  ]
                },
                "openWorldHint": {
                  "description": "If true, this tool may interact with an \"open world\" of external\nentities. If false, the tool's domain of interaction is closed.\nFor example, the world of a web search tool is open, whereas that\nof a memory tool is not.\n\nDefault: true",
                  "type": [
                    "boolean",
                    "null"
                  ]
                }
              }
            },
            "Icon": {
              "description": "A URL pointing to an icon resource or a base64-encoded data URI.\n\nClients that support rendering icons MUST support at least the following MIME types:\n- image/png - PNG images (safe, universal compatibility)\n- image/jpeg (and image/jpg) - JPEG images (safe, universal compatibility)\n\nClients that support rendering icons SHOULD also support:\n- image/svg+xml - SVG images (scalable but requires security precautions)\n- image/webp - WebP images (modern, efficient format)",
              "type": "object",
              "properties": {
                "src": {
                  "description": "A standard URI pointing to an icon resource",
                  "type": "string"
                },
                "mimeType": {
                  "description": "Optional override if the server's MIME type is missing or generic",
                  "type": [
                    "string",
                    "null"
                  ]
                },
                "sizes": {
                  "description": "Size specification (e.g., \"48x48\", \"any\" for SVG, or \"48x48 96x96\")",
                  "type": [
                    "string",
                    "null"
                  ]
                }
              },
              "required": [
                "src"
              ]
            }
          }
        }
        "##);
    }
}
