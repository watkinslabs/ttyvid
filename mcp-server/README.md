# MCP Server for ttyvid

Model Context Protocol (MCP) server that provides ttyvid's complete terminal recording and video generation capabilities to Claude Code and other MCP clients.

## What This Does

This MCP server lets AI assistants like Claude directly:
- **Record terminal sessions** - Direct PTY-based capture to .cast files (asciicast v2 format)
- **Convert recordings to video** - Transform .cast files into GIF or WebM videos
- **Clone terminal appearance** - Auto-detect size, colors, and font for authentic reproduction
- **Use system fonts** - Load TrueType/OpenType fonts with full UTF-8 support
- **Apply themes and styling** - 9 built-in themes with customizable layers and animations
- **List available resources** - Query themes, fonts, and other assets
- **Automate workflows** - Chain recording and conversion for complete automation

## Installation

### From npm

```bash
npm install -g @watkinslabs/ttyvid-mcp
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

### 1. `record_session`

Record a terminal session to a .cast file.

**Parameters:**
- `output` (required): Path to output .cast file
- `shell` (optional): Shell to execute (default: $SHELL or /bin/sh)
- `columns` (optional): Terminal width (default: 80)
- `rows` (optional): Terminal height (default: 24)
- `command` (optional): Command to execute instead of shell

**Example usage in Claude Code:**

```
User: "Record a terminal session to demo.cast with 120 columns"
Claude: [Calls record_session tool with columns=120]
Claude: "Recording started. The shell will capture all output to demo.cast"
```

**Note:** Recording is interactive and requires user input. The recording ends when the shell exits (Ctrl+D or `exit` command).

### 2. `convert_recording`

Convert terminal recordings to video formats.

**Parameters:**
- `input` (required): Path to input .cast file
- `output` (required): Path to output file (.gif or .webm)
- `theme` (optional): Theme name (default, windows7, mac, fdwm, fdwm-x, game, bar, opensource, scripted, simple)
- `font` (optional): Bitmap font name (e.g., IBM_VGA8, Verite_9x16)
- `system_font` (optional): System TrueType font name (e.g., "JetBrains Mono", "monospace", "default")
- `clone` (optional): Auto-detect terminal size, colors, and font (default: false)
- `terminal_colors` (optional): Use terminal's color palette (default: false)
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

### 3. `list_themes`

List all available ttyvid themes.

**Example:**
```
User: "What themes are available?"
Claude: [Calls list_themes]
Claude: "Available themes: default, windows7, mac, fdwm, fdwm-x, game, bar, opensource, scripted, simple"
```

### 4. `list_fonts`

List common available fonts.

### 5. `get_version`

Get ttyvid version information.

## Usage Examples

### Record and Convert Workflow

```
User: "Record a terminal session and convert it to a GIF"
Claude: [Calls record_session to create recording.cast]
Claude: "Recording session to recording.cast. Exit the shell when done."
User: [Interacts with terminal, then exits]
Claude: [Calls convert_recording to create output.gif]
Claude: "Converted recording.cast to output.gif"
```

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

### Record with Custom Dimensions

```
User: "Record a demo session with 120 columns and 30 rows"
Claude: [Calls record_session with columns=120, rows=30]
Claude: "Recording to demo.cast with 120x30 terminal size"
```

### Clone Terminal Appearance

```
User: "Convert recording.cast to output.gif matching my terminal's appearance"
Claude: [Calls convert_recording with clone=true]
Claude: "Created output.gif with auto-detected terminal size, colors, and font"
```

### Use System Font

```
User: "Convert my recording using JetBrains Mono font"
Claude: [Calls convert_recording with system_font="JetBrains Mono"]
Claude: "Converted with TrueType font JetBrains Mono for full UTF-8 support"
```

### Use Terminal Colors

```
User: "Convert recording.cast with my terminal's color scheme"
Claude: [Calls convert_recording with terminal_colors=true]
Claude: "Used your terminal's 16-color palette and default colors"
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

- [ttyvid](https://github.com/watkinslabs/ttyvid) - The terminal video converter
- [MCP](https://modelcontextprotocol.io) - Model Context Protocol specification
- [Claude Code](https://claude.com/claude-code) - AI coding assistant
