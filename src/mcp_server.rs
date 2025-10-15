use anyhow::Result;
use rust_mcp_sdk::{
    mcp_server::{server_runtime, ServerHandler},
    McpServer,
    StdioTransport, TransportOptions,
    schema::{
        CallToolRequest, CallToolResult, Implementation, InitializeResult,
        ListToolsRequest, ListToolsResult, RpcError, ServerCapabilities,
        ServerCapabilitiesTools, TextContent, Tool, ToolInputSchema,
        schema_utils::CallToolError,
    },
};
use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Available themes in ttyvid
const THEMES: &[&str] = &[
    "default",
    "windows7",
    "mac",
    "fdwm",
    "fdwm-x",
    "game",
    "bar",
    "opensource",
    "scripted",
    "simple",
];

/// Available fonts (common subset)
const FONTS: &[&str] = &[
    "IBM_VGA8",
    "IBM_VGA9",
    "IBM_BIOS",
    "IBM_CGA",
    "IBM_EGA_8x14",
    "IBM_EGA_8x8",
    "Verite_8x16",
    "Verite_9x16",
    "ATI_8x16",
    "Phoenix_EGA_8x16",
    "Compaq_Thin_8x16",
];

pub struct TtyvidServerHandler;

fn create_tool(
    name: impl Into<String>,
    description: impl Into<String>,
    required: Vec<String>,
    properties_json: JsonValue,
) -> Tool {
    // Convert JSON properties to HashMap<String, Map<String, Value>>
    let properties: Option<HashMap<String, serde_json::Map<String, JsonValue>>> =
        if let JsonValue::Object(map) = properties_json {
            Some(map.into_iter().map(|(k, v)| {
                if let JsonValue::Object(prop_map) = v {
                    (k, prop_map)
                } else {
                    (k, serde_json::Map::new())
                }
            }).collect())
        } else {
            None
        };

    let input_schema = ToolInputSchema::new(required, properties);

    Tool {
        name: name.into(),
        description: Some(description.into()),
        input_schema,
        annotations: None,
        meta: None,
        output_schema: None,
        title: None,
    }
}

#[async_trait]
impl ServerHandler for TtyvidServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        let tools = vec![
            create_tool(
                "convert_recording",
                "Convert terminal recording (.cast file) to GIF or WebM video",
                vec!["input".to_string(), "output".to_string()],
                serde_json::json!({
                    "input": {
                        "type": "string",
                        "description": "Path to input asciicast (.cast) file"
                    },
                    "output": {
                        "type": "string",
                        "description": "Path to output file (.gif or .webm)"
                    },
                    "theme": {
                        "type": "string",
                        "description": "Theme name (default, windows7, mac, fdwm, fdwm-x, game, bar, opensource, scripted, simple)",
                        "default": "default"
                    },
                    "font": {
                        "type": "string",
                        "description": "Font name (e.g., IBM_VGA8, Verite_9x16)"
                    },
                    "fps": {
                        "type": "number",
                        "description": "Frames per second (3-100)",
                        "default": 10
                    },
                    "speed": {
                        "type": "number",
                        "description": "Speed multiplier",
                        "default": 1.0
                    },
                    "quality": {
                        "type": "number",
                        "description": "Video quality for WebM (0-100, higher is better)",
                        "default": 50
                    },
                    "no_gaps": {
                        "type": "boolean",
                        "description": "Remove gaps in recording",
                        "default": false
                    },
                    "trailer": {
                        "type": "boolean",
                        "description": "Add trailer at end (1.5s pause before loop)",
                        "default": false
                    },
                    "title": {
                        "type": "string",
                        "description": "Title text to display"
                    }
                }),
            ),
            create_tool(
                "list_themes",
                "List all available ttyvid themes",
                vec![],
                serde_json::json!({}),
            ),
            create_tool(
                "list_fonts",
                "List all available ttyvid fonts",
                vec![],
                serde_json::json!({}),
            ),
            create_tool(
                "get_version",
                "Get ttyvid version information",
                vec![],
                serde_json::json!({}),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        let tool_name = request.tool_name();
        let arguments = JsonValue::Object(request.params.arguments.clone().unwrap_or_default());

        match tool_name {
            "convert_recording" => handle_convert_recording(arguments).await,
            "list_themes" => handle_list_themes().await,
            "list_fonts" => handle_list_fonts().await,
            "get_version" => handle_get_version().await,
            _ => Err(CallToolError::unknown_tool(tool_name.to_string())),
        }
    }
}

async fn handle_convert_recording(args: JsonValue) -> Result<CallToolResult, CallToolError> {
    let input = args["input"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("convert_recording", Some("Missing required parameter: input".to_string())))?;
    let output = args["output"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("convert_recording", Some("Missing required parameter: output".to_string())))?;

    let theme = args["theme"].as_str().unwrap_or("default");
    let font = args["font"].as_str();
    let fps = args["fps"].as_u64().unwrap_or(10) as u8;
    let speed = args["speed"].as_f64().unwrap_or(1.0);
    let quality = args["quality"].as_u64().unwrap_or(50) as u8;
    let no_gaps = args["no_gaps"].as_bool().unwrap_or(false);
    let trailer = args["trailer"].as_bool().unwrap_or(false);
    let title = args["title"].as_str();

    // Build command arguments
    let mut cmd_args = vec![
        "-i".to_string(),
        input.to_string(),
        "-o".to_string(),
        output.to_string(),
        "--theme".to_string(),
        theme.to_string(),
        "--fps".to_string(),
        fps.to_string(),
        "--speed".to_string(),
        speed.to_string(),
        "--quality".to_string(),
        quality.to_string(),
    ];

    if let Some(f) = font {
        cmd_args.push("--font".to_string());
        cmd_args.push(f.to_string());
    }
    if no_gaps {
        cmd_args.push("--no-gaps".to_string());
    }
    if trailer {
        cmd_args.push("--trailer".to_string());
    }
    if let Some(t) = title {
        cmd_args.push("--title".to_string());
        cmd_args.push(t.to_string());
    }

    // Execute ttyvid command
    match execute_ttyvid(cmd_args).await {
        Ok((_stdout, _stderr)) => {
            let message = format!(
                "Successfully converted {} to {}\n\nSettings:\n- Theme: {}\n- Font: {}\n- FPS: {}\n- Speed: {}x\n- Quality: {}\n- Remove gaps: {}\n- Trailer: {}\n{}",
                input,
                output,
                theme,
                font.unwrap_or("default"),
                fps,
                speed,
                quality,
                no_gaps,
                trailer,
                title.map(|t| format!("- Title: {}", t)).unwrap_or_default()
            );

            Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
        }
        Err(e) => Err(CallToolError::from_message(format!("ttyvid execution failed: {}", e))),
    }
}

async fn handle_list_themes() -> Result<CallToolResult, CallToolError> {
    let themes_list = THEMES
        .iter()
        .map(|t| format!("- {}", t))
        .collect::<Vec<_>>()
        .join("\n");

    let message = format!(
        "Available ttyvid themes:\n\n{}\n\nUse these theme names with the convert_recording tool.",
        themes_list
    );

    Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
}

async fn handle_list_fonts() -> Result<CallToolResult, CallToolError> {
    let fonts_list = FONTS
        .iter()
        .map(|f| format!("- {}", f))
        .collect::<Vec<_>>()
        .join("\n");

    let message = format!(
        "Available ttyvid fonts (common selection):\n\n{}\n\nMany more fonts are available. Use these font names with the convert_recording tool.",
        fonts_list
    );

    Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
}

async fn handle_get_version() -> Result<CallToolResult, CallToolError> {
    let version = env!("CARGO_PKG_VERSION");
    let message = format!("ttyvid version {}", version);
    Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
}

async fn execute_ttyvid(args: Vec<String>) -> Result<(String, String)> {
    use tokio::process::Command;

    // Try to find ttyvid binary - current binary or in PATH
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.join("ttyvid")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("ttyvid"));

    let output = Command::new(binary)
        .args(&args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to execute ttyvid: {}", e))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "ttyvid exited with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

pub async fn start_mcp_server() -> Result<()> {
    let server_details = InitializeResult {
        protocol_version: "2025-06-18".to_string(),
        server_info: Implementation {
            name: "ttyvid-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            title: Some("ttyvid MCP Server".to_string()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        instructions: Some("MCP server for ttyvid - Convert terminal recordings to video".to_string()),
        meta: None,
    };

    let transport = StdioTransport::new(TransportOptions::default())
        .map_err(|e| anyhow::anyhow!("Failed to create transport: {:?}", e))?;

    let handler = TtyvidServerHandler;

    let server = server_runtime::create_server(server_details, transport, handler);

    eprintln!("ttyvid MCP server running on stdio");
    server.start().await
        .map_err(|e| anyhow::anyhow!("Server error: {:?}", e))?;

    Ok(())
}
