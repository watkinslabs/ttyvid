# MCP Server for ttyvid

Model Context Protocol (MCP) server that provides ttyvid terminal recording conversion tools to Claude Code and other MCP clients.

## What This Does

This MCP server lets AI assistants like Claude directly:
- Convert terminal recordings (.cast files) to GIF/WebM videos
- List available themes and fonts
- Apply custom styling and effects
- Automate video generation workflows

## Installation

### From npm

```bash
npm install -g ttyvid-mcp
```

### From source

```bash
cd mcp-server
npm install
npm run build
npm link
```

## Prerequisites

You must have **ttyvid** installed and available in your PATH:

```bash
cargo install ttyvid
# or build from source
cargo build --release
```

## Configuration

### For Claude Code

Add to your MCP settings (usually `~/.config/claude-code/config.json` or via Settings → MCP):

```json
{
  "mcpServers": {
    "ttyvid": {
      "command": "ttyvid-mcp"
    }
  }
}
```

### For other MCP clients

The server uses stdio transport and can be integrated with any MCP-compatible client:

```json
{
  "command": "node",
  "args": ["/path/to/ttyvid-mcp/dist/index.js"]
}
```

## Available Tools

### 1. `convert_recording`

Convert terminal recordings to video formats.

**Parameters:**
- `input` (required): Path to input .cast file
- `output` (required): Path to output file (.gif or .webm)
- `theme` (optional): Theme name (default, windows7, mac, fdwm, fdwm-x, game, bar, opensource, scripted, simple)
- `font` (optional): Font name (e.g., IBM_VGA8, Verite_9x16)
- `fps` (optional): Frames per second (3-100, default: 10)
- `speed` (optional): Speed multiplier (default: 1.0)
- `quality` (optional): WebM quality 0-100 (default: 50)
- `no_gaps` (optional): Remove gaps in recording (default: false)
- `trailer` (optional): Add 1.5s pause before loop (default: false)
- `title` (optional): Title text to display

**Example usage in Claude Code:**

```
User: "Convert my recording.cast to a GIF with the fdwm-x theme at 30fps"
Claude: [Calls convert_recording tool automatically]
Claude: "Done! Created output.gif with fdwm-x theme at 30fps"
```

### 2. `list_themes`

List all available ttyvid themes.

**Example:**
```
User: "What themes are available?"
Claude: [Calls list_themes]
Claude: "Available themes: default, windows7, mac, fdwm, fdwm-x, game, bar, opensource, scripted, simple"
```

### 3. `list_fonts`

List common available fonts.

### 4. `get_version`

Get ttyvid version information.

## Usage Examples

### Basic Conversion

```
User: "Convert recording.cast to output.gif"
Claude: Uses convert_recording tool
```

### With Custom Theme and Font

```
User: "Convert my demo.cast to demo.gif using the windows7 theme and IBM_VGA8 font"
Claude: Applies theme and font settings automatically
```

### High Quality WebM

```
User: "Convert recording.cast to video.webm at 60fps with high quality"
Claude: Sets fps=60, quality=80, format=webm
```

### With Effects

```
User: "Convert my recording to GIF with the game theme, remove gaps, and add a trailer"
Claude: Applies no_gaps=true, trailer=true
```

## Development

### Build

```bash
npm run build
```

### Watch Mode

```bash
npm run watch
```

### Test Locally

```bash
npm run dev
```

Then test with an MCP client or using stdio.

## How It Works

1. **Server starts** in stdio mode
2. **Client connects** via MCP protocol
3. **Tools are registered** (convert_recording, list_themes, etc.)
4. **Client calls tools** as needed
5. **Server executes** ttyvid commands and returns results

## Architecture

```
MCP Client (Claude Code)
    ↓ MCP Protocol
MCP Server (this)
    ↓ execFile
ttyvid binary
    ↓
Output files (.gif, .webm)
```

## Troubleshooting

### "ttyvid not found"

Make sure ttyvid is installed and in your PATH:
```bash
which ttyvid
# or
cargo install ttyvid
```

### "MCP server not responding"

Check Claude Code's MCP logs:
```bash
# Usually in:
~/.config/claude-code/logs/
```

### "Module not found" errors

Install dependencies:
```bash
npm install
npm run build
```

## License

MIT

## Links

- [ttyvid](https://github.com/chris17453/ttyvid) - The terminal video converter
- [MCP](https://modelcontextprotocol.io) - Model Context Protocol specification
- [Claude Code](https://claude.com/claude-code) - AI coding assistant
