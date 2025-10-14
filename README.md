# ttyvid

Convert terminal recordings to video formats (GIF/WebM). High-performance Rust implementation with full terminal emulation and 56+ embedded classic bitmap fonts.

![ttyvid demo with animated theme layers](demo-preview.gif)

*Example: fdwm-x theme with animated window decoration overlay and 9-slice frame scaling*

![Full-size terminal output example](build_test/test-fullsize.gif)

*Example: Full-size terminal recording*

## Features

- ✅ **Full terminal emulation** - ANSI/VT100 escape sequences
- ✅ **Multiple output formats** - GIF and WebM (with AV1)
- ✅ **56 embedded fonts** - IBM, ATI, Verite, Tandy, Phoenix, and more
- ✅ **Theme system** - Customizable layouts with layers and animations
- ✅ **Asciicast support** - Read .cast v2 files or stdin
- ✅ **Speed control** - Adjust playback speed and FPS
- ✅ **Frame optimization** - Differencing for efficient file sizes

## Installation

### From crates.io

```bash
cargo install ttyvid
```

### From Source

```bash
git clone https://github.com/chris17453/ttyvid.git
cd ttyvid
cargo build --release
```

The binary will be at `target/release/ttyvid`.

### MCP Integration (Model Context Protocol)

Use ttyvid directly from Claude Code and other AI assistants!

**Install MCP Server:**
```bash
cd mcp-server
npm install
npm run build
npm link
```

**Configure Claude Code:**

Add to your MCP settings:
```json
{
  "mcpServers": {
    "ttyvid": {
      "command": "ttyvid-mcp"
    }
  }
}
```

**Use it naturally:**
```
User: "Convert my recording.cast to GIF with fdwm-x theme at 30fps"
Claude: [Automatically converts using ttyvid MCP tool]
Claude: "Done! Created output.gif with fdwm-x theme at 30fps"
```

See [mcp-server/README.md](mcp-server/README.md) for full documentation.

### Quick Start

```bash
# Record a terminal session with asciinema
asciinema rec recording.cast

# Convert to GIF
ttyvid -i recording.cast -o output.gif

# Convert to WebM (requires --features webm)
ttyvid -i recording.cast -o output.webm --format webm
```

## Usage

### From stdin (pipe)

```bash
echo -e "Hello \e[31mRed\e[0m World" | ttyvid -o hello.gif
```

### From asciicast file

```bash
ttyvid -i recording.cast -o output.gif
```

### With custom theme and font

```bash
ttyvid -i recording.cast -o output.gif \
  --theme windows7 \
  --font IBM_VGA8 \
  --fps 30 \
  --speed 1.5
```

### Advanced options

```bash
ttyvid \
  --input recording.cast \
  --output output.gif \
  --theme mac \
  --font Verite_9x16 \
  --fps 30 \
  --speed 2.0 \
  --columns 80 \
  --rows 25 \
  --title "My Demo" \
  --no-gaps \
  --trailer
```

The `--trailer` option adds 1.5 seconds of the final frame at the end before looping, creating a pause effect for better viewing.

## Options

```
Options:
  -i, --input <FILE>          Input asciicast file (reads from stdin if not provided)
  -o, --output <FILE>         Output GIF file
  -t, --theme <THEME>         Theme name or path [default: default]
  -f, --font <FONT>           Font name
      --fps <FPS>             Frames per second (3-100) [default: 10]
      --speed <SPEED>         Speed multiplier [default: 1.0]
  -c, --columns <COLUMNS>     Terminal width in columns
  -r, --rows <ROWS>           Terminal height in rows
  -l, --loop <LOOP>           Number of loops (0 = infinite) [default: 0]
  -d, --delay <DELAY>         Delay before loop restart (milliseconds) [default: 100]
  -g, --no-gaps               Remove gaps in recording
      --trailer               Add trailer at end
      --title <TITLE>         Title text
      --no-autowrap           Disable auto line wrap
      --underlay <UNDERLAY>   Underlay image path
  -h, --help                  Print help
  -V, --version               Print version
```

## Performance

This Rust implementation is significantly faster than the original Python/Cython version:

- **Startup time**: ~50-100ms
- **Frame generation**: Highly optimized with frame differencing
- **Memory usage**: Efficient with object pooling
- **Binary size**: ~5-8 MB (release build with LTO)

## Architecture

### Modules

- **cli**: Command-line argument parsing (clap)
- **input**: stdin and asciicast file readers
- **terminal**: Full VT100/ANSI terminal emulator (vte)
- **renderer**: Text to pixel conversion with bitmap fonts
- **encoder**: GIF encoding with LZW compression and frame differencing
- **theme**: Theme system (simplified for MVP)
- **assets**: Embedded fonts and themes

### Data Flow

```
Input (stdin/cast) → Terminal Emulator →
Renderer → GIF Encoder → Output
```

## Available Fonts

All fonts are automatically embedded at compile time from `themes/fonts/`:

- **IBM**: BIOS, CGA, EGA, VGA, MDA, PS/2, 3270pc, Conv, ISO8/9
- **ATI**: 8x8, 8x14, 8x16, 9x14, 9x16, SmallW_6x8
- **Phoenix**: BIOS, EGA_8x8, EGA_8x14, EGA_8x16, EGA_9x14
- **Compaq**: Thin_8x8, Thin_8x14, Thin_8x16
- **Tandy**: New/Old TV, New/Old 225, New Mono
- **Toshiba**: LCD_8x8, LCD_8x16
- **Verite**: 8x8, 8x14, 8x16, 9x14, 9x16 (default)
- **Wyse**: 700a, 700a-2y, 700b-2y
- **Others**: AMI_BIOS, DTK_BIOS, ITT_BIOS, VTech_BIOS, ATT_PC6300, AmstradPC1512, Kaypro2K, VGA_SquarePx

## Available Themes

Built-in themes in `themes/`:

- `default` - Classic terminal look
- `windows7` - Windows 7 CMD style with dialog frame
- `mac` - macOS Terminal style with window controls
- `fdwm` - Floating window manager theme
- `fdwm-x` - Extended window manager with animations
- `game` - Retro gaming console frame
- `bar` - Status bar theme
- `opensource` - Open source branding banner
- `scripted` - Script demonstration with annotations
- `simple` - Minimal theme (terminal only)

## Theme System

ttyvid features a powerful layer-based theme system that allows extensive customization of the final output with support for animated components, flexible positioning, and multiple scaling modes.

### Layer-Based Architecture

Themes are built from **layers** - individual image files (GIF/PNG) that are composited together:

- **Underlays** (depth < 0): Rendered behind the terminal output (backgrounds, frames)
- **Overlays** (depth ≥ 0): Rendered on top of the terminal output (window controls, decorations)

Each layer can be:
- **Static** or **animated** (multi-frame GIF)
- Positioned anywhere on the canvas
- Scaled using different algorithms
- Animated with custom speed and looping

### Layer Rendering Modes

Layers support multiple rendering modes for different visual effects:

#### Copy Mode
```yaml
mode: copy
```
Direct pixel-perfect copying from source to destination. Ideal for static decorative elements.

#### Tile Mode
```yaml
mode: tile
```
Repeats the layer image across the entire canvas. Perfect for textured backgrounds or patterns.

#### Center Mode
```yaml
mode: center
```
Centers the layer on the canvas. Useful for logos or centered decorative elements.

#### Scale Mode
```yaml
mode: scale
```
Scales the layer to fit the destination bounds using nearest-neighbor interpolation.

#### 9-Slice Scaling
```yaml
mode: 9slice
nineslice:
  outer_left: 10
  outer_top: 10
  outer_right: 10
  outer_bottom: 10
  inner_left: 20
  inner_top: 20
  inner_right: 20
  inner_bottom: 20
```
Advanced scaling that preserves corners and edges while stretching the center. Essential for window frames and dialogs that need to resize without distorting decorative borders.

**How it works:**
- Divides the source image into 9 regions (4 corners, 4 edges, 1 center)
- Corners remain fixed size
- Edges stretch in one direction only
- Center stretches in both directions

#### 3-Slice Scaling
```yaml
mode: 3slice
```
Similar to 9-slice but for horizontal or vertical stretching only. Useful for title bars and status bars.

### Animated Layers

Layers support animated GIF files with full control over playback:

```yaml
layers:
  - depth: -1
    file: layers/animated-background.gif
    mode: center
    animation:
      speed: 1.5        # 1.5x speed multiplier
      loop: true        # Loop animation
      start_frame: 0    # Start at first frame
```

**Animation features:**
- Multi-frame GIF support
- Independent speed control per layer
- Optional looping
- Frame timing preserved from source GIF
- Synchronized with video output timeline

### Layer Positioning

Flexible positioning system with support for absolute and relative coordinates:

```yaml
layers:
  - depth: 1
    file: layers/window-controls.gif
    mode: copy
    dst_bounds:
      left: 10          # 10 pixels from left
      top: 10           # 10 pixels from top
      right: -110       # 110 pixels from right edge (negative = from right)
      bottom: auto      # Auto-calculate based on image size
```

**Positioning options:**
- Absolute pixel coordinates
- Negative values offset from opposite edge
- `auto` for automatic sizing
- Per-layer bounds control

### Custom Theme Example

Create a custom theme with window frame and animated decorations:

```yaml
name: my-custom-theme
background: 0           # Black background
foreground: 7           # White text
transparent: 0          # Transparency color index

padding:
  left: 20
  top: 40
  right: 20
  bottom: 20

title:
  foreground: 15        # Bright white
  background: 4         # Red
  x: 30
  y: 10
  font_size: 1.5

layers:
  # Background frame (underlay)
  - depth: -1
    file: layers/my-frame.gif
    mode: 9slice
    nineslice:
      outer_left: 0
      outer_top: 0
      outer_right: auto
      outer_bottom: auto
      inner_left: 30
      inner_top: 30
      inner_right: auto
      inner_bottom: auto

  # Animated decoration (overlay)
  - depth: 1
    file: layers/spinner.gif
    mode: copy
    animation:
      speed: 2.0
      loop: true
    dst_bounds:
      left: -50
      top: 10
      right: auto
      bottom: auto
```

### Embedded vs Custom Layers

**Embedded layers** (included in binary):
- 11 pre-built layer images
- Loaded automatically by filename
- No external files needed

**Custom layers** (filesystem):
- Place images in `themes/layers/` or specify full path
- Falls back to embedded if not found
- Supports both GIF and PNG formats

### Creating Custom Layers

1. **Design your layer** in any image editor
2. **Export as GIF** (with animation if desired)
3. **Save to** `themes/layers/your-layer.gif`
4. **Reference in theme YAML:**
   ```yaml
   layers:
     - depth: -1
       file: layers/your-layer.gif
       mode: 9slice
   ```

### Theme Customization Workflow

1. Start with an existing theme as template
2. Modify colors, padding, and positioning
3. Add or replace layer images
4. Adjust animation speeds and rendering modes
5. Test with: `ttyvid -i test.cast -o output.gif --theme my-theme`

The theme system is fully extensible - you can mix embedded assets with custom images, create complex multi-layer compositions, and animate individual components independently for professional-looking terminal recordings.

## TODO

- [ ] Parallel frame rendering with rayon
- [ ] SIMD optimizations for bitmap operations
- [ ] Extended color support (256 colors, true color)
- [ ] Additional output formats (MP4, APNG)
- [ ] GPU acceleration exploration

## Comparison with Original

| Feature | Original (Python) | ttyvid (Rust) | Status |
|---------|------------------|--------------|--------|
| Terminal Emulation | Custom | Full ANSI/VT100 | ✅ |
| GIF Encoding | Custom | gif crate | ✅ |
| WebM Encoding | N/A | rav1e (AV1) | ✅ |
| Font Support | 50+ .fd fonts | 56 .fd fonts embedded | ✅ |
| Theme System | Full with layers | Full with layers | ✅ |
| Performance | Good | Excellent | ✅ |
| Binary Size | N/A (Python) | ~5-8 MB | ✅ |
| Dependencies | Python + libs | None (static) | ✅ |

## Building from Source

### Requirements

- Rust 1.70+ (2021 edition)
- Cargo

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build with WebM support (requires rav1e)
cargo build --release --features webm

# Run tests
cargo test

# Run with example
echo "Test" | cargo run --release -- -o test.gif
```

### Build System

The project uses `build.rs` to automatically:
- Scan `themes/fonts/` directory
- Generate font embedding code at compile time
- Create font lookup tables

To add new fonts, simply place `.fd` files in `themes/fonts/` and rebuild.

## License

MIT

## Credits

- Original ttygif concept and fonts
- Rust implementation with modern architecture
- gif crate for GIF encoding
- rav1e for WebM/AV1 encoding

## Contributing

Contributions welcome! Areas of interest:

- Performance optimizations (SIMD, GPU, parallel)
- Additional output formats
- Extended color support (true color)
- Bug fixes and documentation
