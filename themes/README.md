# TTYGif Themes

Themes are YAML files that define the visual appearance of the terminal output, including backgrounds, borders, overlays, and color palettes.

## Theme Structure

```yaml
name: theme_name
background: 0              # Default background color index
foreground: 7              # Default foreground color index
default_background: 0
default_foreground: 7
transparent: 0             # Transparent color index

title:                     # Optional title text configuration
  foreground: 16
  background: 0
  x: 30                    # X position in pixels
  y: 5                     # Y position in pixels
  font_size: 1.5           # Font scale multiplier

padding:                   # Optional padding/margins
  left: 10
  top: 36
  right: 10
  bottom: 10

layers:                    # Image layers (backgrounds/overlays)
  - depth: -2              # Negative = underlay, positive = overlay
    file: "background.gif" # Path to image file
    mode: "9slice"         # Rendering mode (see below)
    nineslice:             # 9-slice configuration (for mode: 9slice)
      outer_left: 0
      outer_top: 0
      outer_right: 100
      outer_bottom: 100
      inner_left: 10
      inner_top: 10
      inner_right: 90
      inner_bottom: 90
    dst_bounds:            # Destination bounds
      left: 0
      top: 0
      right: auto          # "auto" calculates from image
      bottom: auto
    copy_mode: tile        # How to fill (copy or tile)

palette:                   # Optional custom color palette
  colors: 256
  rgb:
    - [0, 0, 0]           # RGB values for each color
    - [255, 0, 0]
    # ... up to 256 colors
```

## Layer Modes

- **none**: Place image as-is at coordinates
- **center**: Center the image
- **stretch**: Stretch to fill (may distort aspect ratio)
- **tile**: Tile the image starting at 0,0
- **9slice**: Scale using 9-slice technique (preserves corners/edges)
- **copy**: Copy from source bounds to dest bounds

## 9-Slice Scaling

9-slice divides an image into 9 sections:
```
+---+-------+---+
| 1 |   2   | 3 |  <- outer/inner top
+---+-------+---+
| 4 |   5   | 6 |
+---+-------+---+
| 7 |   8   | 9 |  <- outer/inner bottom
+---+-------+---+
^   ^       ^   ^
|   |       |   |
o   i       i   o
u   n       n   u
t   n       n   t
e   e       e   e
r   r       r   r

```

- Corners (1,3,7,9) are preserved
- Edges (2,4,6,8) stretch in one direction
- Center (5) stretches in both directions

## Bound Values

Bounds can be:
- **Integer**: Specific pixel value
- **auto**: Calculate automatically from image dimensions

## Layer Depth

- **Negative** (-2, -1): Underlays (drawn behind terminal)
- **Positive** (1, 2): Overlays (drawn on top of terminal)
- Layers are drawn in depth order (lowest to highest)

## Example Themes

- `default.yaml` - Plain terminal, no decorations
- `windows7.yaml` - Windows 7 dialog window style with 9-slice borders

## Creating Custom Themes

1. Create a new `.yaml` file in the `themes/` directory
2. Add your background/overlay images
3. Configure layers with appropriate modes
4. Test with: `ttygif-rust --theme path/to/theme.yaml`
