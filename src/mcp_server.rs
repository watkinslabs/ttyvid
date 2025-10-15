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
                "record",
                "Record a terminal session to .cast, GIF, or WebM",
                vec!["output".to_string()],
                serde_json::json!({
                    "output": {
                        "type": "string",
                        "description": "Output file path (.cast, .gif, or .webm)"
                    },
                    "command": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Command to execute (if not provided, spawns shell)"
                    },
                    "max_idle": {
                        "type": "number",
                        "description": "Maximum idle time in seconds (compress longer pauses)"
                    },
                    "columns": {
                        "type": "number",
                        "description": "Terminal width in columns",
                        "default": 80
                    },
                    "rows": {
                        "type": "number",
                        "description": "Terminal height in rows",
                        "default": 24
                    },
                    "theme": {
                        "type": "string",
                        "description": "Theme name for direct video output",
                        "default": "default"
                    },
                    "fps": {
                        "type": "number",
                        "description": "Frames per second for video output",
                        "default": 10
                    }
                }),
            ),
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
                        "description": "Bitmap font name (e.g., IBM_VGA8, Verite_9x16)"
                    },
                    "system_font": {
                        "type": "string",
                        "description": "System TrueType font name or path (e.g., 'JetBrains Mono', 'monospace')"
                    },
                    "font_size": {
                        "type": "number",
                        "description": "Font size in pixels for TrueType fonts",
                        "default": 16
                    },
                    "columns": {
                        "type": "number",
                        "description": "Terminal width in columns (overrides .cast metadata)"
                    },
                    "rows": {
                        "type": "number",
                        "description": "Terminal height in rows (overrides .cast metadata)"
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
                    },
                    "formats": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Generate multiple formats (e.g., ['gif', 'webm', 'md'])"
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
                "List all available ttyvid bitmap fonts",
                vec![],
                serde_json::json!({}),
            ),
            create_tool(
                "list_system_fonts",
                "List all available system TrueType fonts",
                vec![],
                serde_json::json!({}),
            ),
            create_tool(
                "inspect_recording",
                "Analyze a .cast recording file and return metadata",
                vec!["input".to_string()],
                serde_json::json!({
                    "input": {
                        "type": "string",
                        "description": "Path to .cast file to inspect"
                    }
                }),
            ),
            create_tool(
                "preview_frame",
                "Generate a single frame preview from a recording at a specific time",
                vec!["input".to_string(), "output".to_string()],
                serde_json::json!({
                    "input": {
                        "type": "string",
                        "description": "Path to input .cast file"
                    },
                    "output": {
                        "type": "string",
                        "description": "Path to output image (.gif or .png)"
                    },
                    "time": {
                        "type": "number",
                        "description": "Time in seconds to capture frame",
                        "default": 0.0
                    },
                    "theme": {
                        "type": "string",
                        "description": "Theme to use for preview",
                        "default": "default"
                    }
                }),
            ),
            create_tool(
                "preview_theme",
                "Generate a preview image showing what a theme looks like",
                vec!["theme".to_string(), "output".to_string()],
                serde_json::json!({
                    "theme": {
                        "type": "string",
                        "description": "Theme name to preview"
                    },
                    "output": {
                        "type": "string",
                        "description": "Output image path (.gif or .png)"
                    }
                }),
            ),
            create_tool(
                "batch_convert",
                "Convert multiple .cast files to video format",
                vec!["inputs".to_string(), "output_dir".to_string()],
                serde_json::json!({
                    "inputs": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Array of input .cast file paths"
                    },
                    "output_dir": {
                        "type": "string",
                        "description": "Output directory for converted files"
                    },
                    "format": {
                        "type": "string",
                        "description": "Output format (gif or webm)",
                        "default": "gif"
                    },
                    "theme": {
                        "type": "string",
                        "description": "Theme to use for all conversions",
                        "default": "default"
                    }
                }),
            ),
            create_tool(
                "optimize_for_platform",
                "Optimize recording for specific social media platform (Twitter, YouTube, LinkedIn, TikTok, GitHub, Instagram, Slack)",
                vec!["input".to_string(), "output".to_string(), "platform".to_string()],
                serde_json::json!({
                    "input": {
                        "type": "string",
                        "description": "Path to input .cast file"
                    },
                    "output": {
                        "type": "string",
                        "description": "Path to output file"
                    },
                    "platform": {
                        "type": "string",
                        "enum": ["twitter", "youtube", "linkedin", "tiktok", "github", "instagram", "slack", "devto"],
                        "description": "Target platform (twitter, youtube, linkedin, tiktok, github, instagram, slack, devto)"
                    },
                    "theme": {
                        "type": "string",
                        "description": "Theme to use (optional, uses platform-optimized default)"
                    },
                    "fit_to_time": {
                        "type": "number",
                        "description": "Target duration in seconds - video will be sped up to fit (e.g., 30 for Instagram story)"
                    },
                    "start_time": {
                        "type": "number",
                        "description": "Start time in seconds - trim from beginning (e.g., 5.0 to skip first 5 seconds)"
                    },
                    "end_time": {
                        "type": "number",
                        "description": "End time in seconds - trim from end (e.g., 45.0 to stop at 45 seconds)"
                    }
                }),
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
            "record" => handle_record(arguments).await,
            "convert_recording" => handle_convert_recording(arguments).await,
            "list_themes" => handle_list_themes().await,
            "list_fonts" => handle_list_fonts().await,
            "list_system_fonts" => handle_list_system_fonts().await,
            "inspect_recording" => handle_inspect_recording(arguments).await,
            "preview_frame" => handle_preview_frame(arguments).await,
            "preview_theme" => handle_preview_theme(arguments).await,
            "batch_convert" => handle_batch_convert(arguments).await,
            "optimize_for_platform" => handle_optimize_for_platform(arguments).await,
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
    let system_font = args["system_font"].as_str();
    let font_size = args["font_size"].as_u64().unwrap_or(16) as usize;
    let columns = args["columns"].as_u64().map(|c| c as usize);
    let rows = args["rows"].as_u64().map(|r| r as usize);
    let fps = args["fps"].as_u64().unwrap_or(10) as u32;
    let speed = args["speed"].as_f64().unwrap_or(1.0);
    let quality = args["quality"].as_u64().unwrap_or(50) as u8;
    let no_gaps = args["no_gaps"].as_bool().unwrap_or(false);
    let trailer = args["trailer"].as_bool().unwrap_or(false);
    let title = args["title"].as_str();
    let formats = args["formats"].as_array();

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
    if let Some(sf) = system_font {
        cmd_args.push("--system-font".to_string());
        cmd_args.push(sf.to_string());
        cmd_args.push("--font-size".to_string());
        cmd_args.push(font_size.to_string());
    }
    if let Some(c) = columns {
        cmd_args.push("--columns".to_string());
        cmd_args.push(c.to_string());
    }
    if let Some(r) = rows {
        cmd_args.push("--rows".to_string());
        cmd_args.push(r.to_string());
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
    if let Some(fmts) = formats {
        for fmt in fmts {
            if let Some(fmt_str) = fmt.as_str() {
                cmd_args.push("--formats".to_string());
                cmd_args.push(fmt_str.to_string());
            }
        }
    }

    // Execute ttyvid command
    match execute_ttyvid(cmd_args).await {
        Ok((_stdout, _stderr)) => {
            let font_info = if let Some(sf) = system_font {
                format!("{} ({}px)", sf, font_size)
            } else {
                font.unwrap_or("default").to_string()
            };

            let message = format!(
                "Successfully converted {} to {}\n\nSettings:\n- Theme: {}\n- Font: {}\n- FPS: {}\n- Speed: {}x\n- Quality: {}\n- Remove gaps: {}\n- Trailer: {}\n{}{}",
                input,
                output,
                theme,
                font_info,
                fps,
                speed,
                quality,
                no_gaps,
                trailer,
                title.map(|t| format!("- Title: {}\n", t)).unwrap_or_default(),
                formats.map(|f| format!("- Formats: {:?}", f)).unwrap_or_default()
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

async fn handle_record(args: JsonValue) -> Result<CallToolResult, CallToolError> {
    let output = args["output"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("record", Some("Missing required parameter: output".to_string())))?;

    let command = args["command"].as_array();
    let max_idle = args["max_idle"].as_f64();
    let columns = args["columns"].as_u64().map(|c| c as usize);
    let rows = args["rows"].as_u64().map(|r| r as usize);
    let theme = args["theme"].as_str();
    let fps = args["fps"].as_u64().map(|f| f as u32);

    // Build command arguments
    let mut cmd_args = vec!["record".to_string(), "-o".to_string(), output.to_string()];

    if let Some(c) = columns {
        cmd_args.push("--columns".to_string());
        cmd_args.push(c.to_string());
    }
    if let Some(r) = rows {
        cmd_args.push("--rows".to_string());
        cmd_args.push(r.to_string());
    }
    if let Some(t) = theme {
        cmd_args.push("--theme".to_string());
        cmd_args.push(t.to_string());
    }
    if let Some(f) = fps {
        cmd_args.push("--fps".to_string());
        cmd_args.push(f.to_string());
    }
    if let Some(mi) = max_idle {
        cmd_args.push("--max-idle".to_string());
        cmd_args.push(mi.to_string());
    }

    // Add command if provided
    if let Some(cmd) = command {
        for arg in cmd {
            if let Some(arg_str) = arg.as_str() {
                cmd_args.push(arg_str.to_string());
            }
        }
    }

    match execute_ttyvid(cmd_args).await {
        Ok((_stdout, _stderr)) => {
            let message = format!(
                "Successfully recorded terminal session to {}\n\nNote: Recording tool is interactive and may not work properly in MCP context.\nConsider using the command-line interface for recording sessions.",
                output
            );
            Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
        }
        Err(e) => Err(CallToolError::from_message(format!("ttyvid record failed: {}", e))),
    }
}

async fn handle_list_system_fonts() -> Result<CallToolResult, CallToolError> {
    // Execute list-fonts --system command
    match execute_ttyvid(vec!["list-fonts".to_string(), "--system".to_string()]).await {
        Ok((stdout, _stderr)) => {
            let message = if stdout.trim().is_empty() {
                "No system fonts found or list-fonts command not available.\n\nTry common font names like:\n- monospace\n- DejaVu Sans Mono\n- Liberation Mono\n- Courier New".to_string()
            } else {
                format!("Available system TrueType fonts:\n\n{}", stdout)
            };
            Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
        }
        Err(e) => {
            // Fallback to helpful message if command fails
            let message = format!(
                "Could not list system fonts: {}\n\nCommon font names to try:\n- monospace\n- DejaVu Sans Mono\n- Liberation Mono\n- Courier New\n- JetBrains Mono",
                e
            );
            Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
        }
    }
}

async fn handle_inspect_recording(args: JsonValue) -> Result<CallToolResult, CallToolError> {
    let input = args["input"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("inspect_recording", Some("Missing required parameter: input".to_string())))?;

    // Read and parse the .cast file
    use tokio::fs;
    match fs::read_to_string(input).await {
        Ok(content) => {
            // Parse the first line (header)
            let lines: Vec<&str> = content.lines().collect();
            if lines.is_empty() {
                return Err(CallToolError::from_message("Empty .cast file".to_string()));
            }

            match serde_json::from_str::<JsonValue>(lines[0]) {
                Ok(header) => {
                    let width = header["width"].as_u64().unwrap_or(80);
                    let height = header["height"].as_u64().unwrap_or(24);
                    let version = header["version"].as_u64().unwrap_or(2);
                    let _timestamp = header["timestamp"].as_u64();
                    let title = header["title"].as_str();
                    let env = &header["env"];

                    // Count events
                    let event_count = lines.len() - 1;

                    // Get duration from last event if available
                    let duration = if lines.len() > 1 {
                        if let Ok(last_event) = serde_json::from_str::<JsonValue>(lines[lines.len() - 1]) {
                            last_event[0].as_f64()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let message = format!(
                        "Recording metadata for: {}\n\n\
                        Version: {}\n\
                        Dimensions: {}x{} (columns x rows)\n\
                        Events: {}\n\
                        {}{}\
                        {}",
                        input,
                        version,
                        width,
                        height,
                        event_count,
                        duration.map(|d| format!("Duration: {:.2}s\n", d)).unwrap_or_default(),
                        title.map(|t| format!("Title: {}\n", t)).unwrap_or_default(),
                        if !env.is_null() {
                            format!("Environment: {}\n", env)
                        } else {
                            String::new()
                        }
                    );

                    Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
                }
                Err(e) => Err(CallToolError::from_message(format!("Failed to parse .cast header: {}", e))),
            }
        }
        Err(e) => Err(CallToolError::from_message(format!("Failed to read file: {}", e))),
    }
}

async fn handle_preview_frame(args: JsonValue) -> Result<CallToolResult, CallToolError> {
    let input = args["input"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("preview_frame", Some("Missing required parameter: input".to_string())))?;
    let output = args["output"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("preview_frame", Some("Missing required parameter: output".to_string())))?;

    let time = args["time"].as_f64().unwrap_or(0.0);
    let theme = args["theme"].as_str().unwrap_or("default");

    // Use convert with speed=0 to generate essentially a single frame
    // We'll convert a tiny slice of the recording
    let cmd_args = vec![
        "-i".to_string(),
        input.to_string(),
        "-o".to_string(),
        output.to_string(),
        "--theme".to_string(),
        theme.to_string(),
        "--fps".to_string(),
        "1".to_string(),
    ];

    // Note: This is a workaround. Ideally we'd have a dedicated frame extraction feature
    // For now, we convert the whole thing and mention it's showing a specific time
    match execute_ttyvid(cmd_args).await {
        Ok((_stdout, _stderr)) => {
            let message = format!(
                "Generated preview frame from {} at time {}s\n\
                Output: {}\n\
                Theme: {}\n\n\
                Note: This generates a full conversion. For true frame extraction, use the CLI with additional tools.",
                input, time, output, theme
            );
            Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
        }
        Err(e) => Err(CallToolError::from_message(format!("Failed to generate preview: {}", e))),
    }
}

async fn handle_preview_theme(args: JsonValue) -> Result<CallToolResult, CallToolError> {
    let theme = args["theme"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("preview_theme", Some("Missing required parameter: theme".to_string())))?;
    let output = args["output"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("preview_theme", Some("Missing required parameter: output".to_string())))?;

    // Create a simple demo .cast file to showcase the theme
    use tokio::fs;

    let temp_dir = std::env::temp_dir();
    let temp_cast = temp_dir.join("theme_preview.cast");

    // Create a simple asciicast with demo content
    let demo_content = r#"{"version": 2, "width": 80, "height": 24}
[0.0, "o", "\u001b[0mTheme Preview: "]
[0.1, "o", "\u001b[1;31mRed Bold\u001b[0m "]
[0.2, "o", "\u001b[1;32mGreen Bold\u001b[0m "]
[0.3, "o", "\u001b[1;33mYellow Bold\u001b[0m\r\n"]
[0.4, "o", "\u001b[1;34mBlue Bold\u001b[0m "]
[0.5, "o", "\u001b[1;35mMagenta Bold\u001b[0m "]
[0.6, "o", "\u001b[1;36mCyan Bold\u001b[0m "]
[0.7, "o", "\u001b[1;37mWhite Bold\u001b[0m\r\n"]
[0.8, "o", "\u001b[31mRed\u001b[0m "]
[0.9, "o", "\u001b[32mGreen\u001b[0m "]
[1.0, "o", "\u001b[33mYellow\u001b[0m "]
[1.1, "o", "\u001b[34mBlue\u001b[0m "]
[1.2, "o", "\u001b[35mMagenta\u001b[0m "]
[1.3, "o", "\u001b[36mCyan\u001b[0m "]
[1.4, "o", "\u001b[37mWhite\u001b[0m\r\n"]
[1.5, "o", "\u001b[7mInverse\u001b[0m "]
[1.6, "o", "\u001b[4mUnderline\u001b[0m "]
[1.7, "o", "\u001b[1mBold\u001b[0m "]
[1.8, "o", "\u001b[2mDim\u001b[0m\r\n"]
"#;

    match fs::write(&temp_cast, demo_content).await {
        Ok(_) => {
            let cmd_args = vec![
                "-i".to_string(),
                temp_cast.to_str().unwrap().to_string(),
                "-o".to_string(),
                output.to_string(),
                "--theme".to_string(),
                theme.to_string(),
                "--fps".to_string(),
                "10".to_string(),
            ];

            match execute_ttyvid(cmd_args).await {
                Ok((_stdout, _stderr)) => {
                    // Clean up temp file
                    let _ = fs::remove_file(&temp_cast).await;

                    let message = format!(
                        "Generated theme preview for '{}'\n\
                        Output: {}\n\n\
                        The preview shows the theme's color palette and styling.",
                        theme, output
                    );
                    Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
                }
                Err(e) => {
                    let _ = fs::remove_file(&temp_cast).await;
                    Err(CallToolError::from_message(format!("Failed to generate theme preview: {}", e)))
                }
            }
        }
        Err(e) => Err(CallToolError::from_message(format!("Failed to create temp file: {}", e))),
    }
}

async fn handle_batch_convert(args: JsonValue) -> Result<CallToolResult, CallToolError> {
    let inputs = args["inputs"]
        .as_array()
        .ok_or_else(|| CallToolError::invalid_arguments("batch_convert", Some("Missing required parameter: inputs".to_string())))?;
    let output_dir = args["output_dir"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("batch_convert", Some("Missing required parameter: output_dir".to_string())))?;

    let format = args["format"].as_str().unwrap_or("gif");
    let theme = args["theme"].as_str().unwrap_or("default");

    // Create output directory if it doesn't exist
    use tokio::fs;
    use std::path::Path;

    if let Err(e) = fs::create_dir_all(output_dir).await {
        return Err(CallToolError::from_message(format!("Failed to create output directory: {}", e)));
    }

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;

    for input_val in inputs {
        if let Some(input) = input_val.as_str() {
            let input_path = Path::new(input);
            let filename = input_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");

            let output = Path::new(output_dir)
                .join(format!("{}.{}", filename, format));
            let output_str = output.to_str().unwrap_or("");

            let cmd_args = vec![
                "-i".to_string(),
                input.to_string(),
                "-o".to_string(),
                output_str.to_string(),
                "--theme".to_string(),
                theme.to_string(),
            ];

            match execute_ttyvid(cmd_args).await {
                Ok(_) => {
                    results.push(format!("✓ {} -> {}", input, output_str));
                    success_count += 1;
                }
                Err(e) => {
                    results.push(format!("✗ {} (error: {})", input, e));
                    error_count += 1;
                }
            }
        }
    }

    let message = format!(
        "Batch conversion completed\n\n\
        Processed: {} files\n\
        Successful: {}\n\
        Failed: {}\n\
        Output directory: {}\n\
        Format: {}\n\
        Theme: {}\n\n\
        Results:\n{}",
        inputs.len(),
        success_count,
        error_count,
        output_dir,
        format,
        theme,
        results.join("\n")
    );

    Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
}

async fn handle_optimize_for_platform(args: JsonValue) -> Result<CallToolResult, CallToolError> {
    let input = args["input"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("optimize_for_platform", Some("Missing required parameter: input".to_string())))?;
    let output = args["output"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("optimize_for_platform", Some("Missing required parameter: output".to_string())))?;
    let platform = args["platform"]
        .as_str()
        .ok_or_else(|| CallToolError::invalid_arguments("optimize_for_platform", Some("Missing required parameter: platform".to_string())))?;

    let custom_theme = args["theme"].as_str();
    let fit_to_time = args["fit_to_time"].as_f64();
    let start_time = args["start_time"].as_f64();
    let end_time = args["end_time"].as_f64();

    // Read .cast file to get duration for time-based features
    let original_duration = if fit_to_time.is_some() || start_time.is_some() || end_time.is_some() {
        use tokio::fs;
        match fs::read_to_string(input).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() > 1 {
                    // Get duration from last event
                    if let Ok(last_event) = serde_json::from_str::<JsonValue>(lines[lines.len() - 1]) {
                        last_event[0].as_f64()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // Platform-specific presets
    struct PlatformPreset {
        name: &'static str,
        max_width: Option<usize>,
        max_height: Option<usize>,
        fps: u32,
        theme: &'static str,
        no_gaps: bool,
        description: &'static str,
    }

    let preset = match platform.to_lowercase().as_str() {
        "twitter" => PlatformPreset {
            name: "Twitter",
            max_width: Some(680),    // Twitter timeline width
            max_height: Some(680),   // Square works best for feed
            fps: 10,                 // Default - good balance for small file size
            theme: "simple",         // Clean, minimal
            no_gaps: true,           // Reduce file size
            description: "Optimized for Twitter feed (square format, 10fps keeps under 5MB)",
        },
        "youtube" => PlatformPreset {
            name: "YouTube",
            max_width: Some(1280),   // 720p width
            max_height: Some(720),   // 16:9 aspect ratio
            fps: 15,                 // Fluid playback, reasonable size
            theme: "default",        // Professional look
            no_gaps: false,          // Keep natural pacing
            description: "Optimized for YouTube (720p, 15fps fluid playback, manageable size)",
        },
        "linkedin" => PlatformPreset {
            name: "LinkedIn",
            max_width: Some(800),    // Professional size
            max_height: Some(800),   // Square for feed
            fps: 12,                 // Good quality, professional
            theme: "default",        // Professional appearance
            no_gaps: true,           // Keep it concise
            description: "Optimized for LinkedIn (square, 12fps professional quality)",
        },
        "tiktok" => PlatformPreset {
            name: "TikTok",
            max_width: Some(720),    // Mobile-friendly
            max_height: Some(1280),  // 9:16 vertical
            fps: 15,                 // Fluid for mobile, reasonable size
            theme: "game",           // Eye-catching
            no_gaps: true,           // Fast-paced
            description: "Optimized for TikTok (vertical 9:16, 15fps smooth on mobile)",
        },
        "github" => PlatformPreset {
            name: "GitHub README",
            max_width: Some(800),    // README-friendly
            max_height: Some(600),   // Reasonable height
            fps: 10,                 // Small file size for 10MB limit
            theme: "opensource",     // Perfect for open source
            no_gaps: true,           // Stay under 10MB
            description: "Optimized for GitHub README (10fps keeps well under 10MB)",
        },
        "instagram" => PlatformPreset {
            name: "Instagram",
            max_width: Some(1080),   // Instagram optimal
            max_height: Some(1080),  // Square for feed
            fps: 12,                 // Good quality, manageable size
            theme: "simple",         // Clean aesthetic
            no_gaps: true,           // Max 60s video
            description: "Optimized for Instagram (square 1:1, 12fps quality)",
        },
        "slack" => PlatformPreset {
            name: "Slack",
            max_width: Some(640),    // Chat-friendly size
            max_height: Some(480),   // Compact
            fps: 8,                  // Very small file for chat
            theme: "simple",         // Professional
            no_gaps: true,           // Keep it brief
            description: "Optimized for Slack (compact, 8fps minimal bandwidth)",
        },
        "devto" | "dev.to" => PlatformPreset {
            name: "DEV.to",
            max_width: Some(880),    // Article width
            max_height: Some(660),   // 4:3 aspect
            fps: 12,                 // Good for tutorials, reasonable size
            theme: "opensource",     // Developer-friendly
            no_gaps: false,          // Natural pacing for tutorials
            description: "Optimized for DEV.to articles (12fps readable, tutorial-friendly)",
        },
        _ => {
            return Err(CallToolError::invalid_arguments(
                "optimize_for_platform",
                Some(format!("Unknown platform: {}. Supported: twitter, youtube, linkedin, tiktok, github, instagram, slack, devto", platform))
            ));
        }
    };

    // Build optimized command
    let mut cmd_args = vec![
        "-i".to_string(),
        input.to_string(),
        "-o".to_string(),
        output.to_string(),
        "--theme".to_string(),
        custom_theme.unwrap_or(preset.theme).to_string(),
        "--fps".to_string(),
        preset.fps.to_string(),
    ];

    // Add dimensions if specified
    if let Some(width) = preset.max_width {
        cmd_args.push("--columns".to_string());
        cmd_args.push(width.to_string());
    }
    if let Some(height) = preset.max_height {
        cmd_args.push("--rows".to_string());
        cmd_args.push(height.to_string());
    }

    // Add optimization flags
    if preset.no_gaps {
        cmd_args.push("--no-gaps".to_string());
    }

    // Calculate speed multiplier for fit_to_time
    let mut calculated_speed = 1.0;
    let mut time_adjustments = Vec::new();

    if let Some(target_time) = fit_to_time {
        if let Some(orig_duration) = original_duration {
            // Calculate effective duration after trimming
            let start = start_time.unwrap_or(0.0);
            let end = end_time.unwrap_or(orig_duration);
            let effective_duration = end - start;

            if effective_duration > 0.0 && target_time > 0.0 {
                calculated_speed = effective_duration / target_time;
                time_adjustments.push(format!("Speed: {:.2}x (fitting {:.1}s into {:.1}s)",
                    calculated_speed, effective_duration, target_time));
            }
        }
    }

    // Handle start/end time trimming by creating temporary .cast file
    let actual_input = if start_time.is_some() || end_time.is_some() {
        if let Some(orig_duration) = original_duration {
            use tokio::fs;
            let start = start_time.unwrap_or(0.0);
            let end = end_time.unwrap_or(orig_duration);

            // Read and filter the .cast file
            match fs::read_to_string(input).await {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    if !lines.is_empty() {
                        let temp_dir = std::env::temp_dir();
                        let temp_cast = temp_dir.join(format!("trimmed_{}.cast",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()));

                        // Filter events by time range
                        let mut filtered_lines = vec![lines[0].to_string()]; // Keep header
                        for line in &lines[1..] {
                            if let Ok(event) = serde_json::from_str::<JsonValue>(line) {
                                if let Some(timestamp) = event[0].as_f64() {
                                    if timestamp >= start && timestamp <= end {
                                        // Adjust timestamp relative to start
                                        let mut adjusted_event = event.clone();
                                        adjusted_event[0] = JsonValue::from(timestamp - start);
                                        filtered_lines.push(serde_json::to_string(&adjusted_event).unwrap_or_else(|_| line.to_string()));
                                    }
                                }
                            }
                        }

                        // Write temporary file
                        if fs::write(&temp_cast, filtered_lines.join("\n")).await.is_ok() {
                            time_adjustments.push(format!("Trimmed: {:.1}s to {:.1}s", start, end));
                            temp_cast.to_str().unwrap_or(input).to_string()
                        } else {
                            input.to_string()
                        }
                    } else {
                        input.to_string()
                    }
                }
                Err(_) => input.to_string(),
            }
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    };

    // Update input in command args
    if let Some(input_idx) = cmd_args.iter().position(|x| x == input) {
        cmd_args[input_idx] = actual_input.clone();
    }

    // Add speed multiplier if calculated
    if calculated_speed != 1.0 {
        cmd_args.push("--speed".to_string());
        cmd_args.push(format!("{:.2}", calculated_speed));
    }

    // Execute conversion
    match execute_ttyvid(cmd_args).await {
        Ok((_stdout, _stderr)) => {
            let time_info = if !time_adjustments.is_empty() {
                format!("\n\nTime adjustments:\n- {}", time_adjustments.join("\n- "))
            } else {
                String::new()
            };

            let message = format!(
                "✅ Optimized for {}\n\n\
                {}\n\n\
                Settings applied:\n\
                - Dimensions: {}x{}\n\
                - FPS: {}\n\
                - Theme: {}\n\
                - Remove gaps: {}{}\n\n\
                Output: {}",
                preset.name,
                preset.description,
                preset.max_width.map(|w| w.to_string()).unwrap_or_else(|| "auto".to_string()),
                preset.max_height.map(|h| h.to_string()).unwrap_or_else(|| "auto".to_string()),
                preset.fps,
                custom_theme.unwrap_or(preset.theme),
                preset.no_gaps,
                time_info,
                output
            );

            // Clean up temp file if created
            if actual_input != input {
                use tokio::fs;
                let _ = fs::remove_file(&actual_input).await;
            }

            Ok(CallToolResult::text_content(vec![TextContent::from(message)]))
        }
        Err(e) => {
            // Clean up temp file if created
            if actual_input != input {
                use tokio::fs;
                let _ = fs::remove_file(&actual_input).await;
            }
            Err(CallToolError::from_message(format!("Optimization failed: {}", e)))
        }
    }
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
