// GPU compute shader for terminal frame rendering
// Each thread renders one pixel of the output canvas

struct Cell {
    character: u32,
    fg_color: u32,
    bg_color: u32,
    flags: u32,
}

struct RenderParams {
    canvas_width: u32,
    canvas_height: u32,
    grid_width: u32,
    grid_height: u32,
    cell_width: u32,
    cell_height: u32,
    font_data_stride: u32,
    padding: u32,
}

@group(0) @binding(0) var<storage, read> grid_cells: array<Cell>;
@group(0) @binding(1) var<storage, read> font_data: array<u32>;
@group(0) @binding(2) var<storage, read> palette: array<u32>;
@group(0) @binding(3) var<storage, read_write> output: array<u32>;
@group(0) @binding(4) var<uniform> params: RenderParams;

const REVERSE_FLAG: u32 = 1u;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let px = global_id.x;
    let py = global_id.y;

    // Bounds check
    if (px >= params.canvas_width || py >= params.canvas_height) {
        return;
    }

    // Determine which cell this pixel belongs to
    let cell_col = px / params.cell_width;
    let cell_row = py / params.cell_height;

    if (cell_col >= params.grid_width || cell_row >= params.grid_height) {
        return;
    }

    // Get cell data
    let cell_idx = cell_row * params.grid_width + cell_col;
    let cell = grid_cells[cell_idx];

    // Pixel position within the cell
    let gx = px % params.cell_width;
    let gy = py % params.cell_height;

    // Get glyph bitmap (intensity 0-10 scale)
    let char_offset = cell.character * params.cell_width * params.cell_height;
    let glyph_idx = char_offset + gy * params.cell_width + gx;
    let intensity = font_data[glyph_idx];

    // Handle reverse flag
    var fg_color = cell.fg_color;
    var bg_color = cell.bg_color;
    if ((cell.flags & REVERSE_FLAG) != 0u) {
        let temp = fg_color;
        fg_color = bg_color;
        bg_color = temp;
    }

    // Select color based on intensity
    // Intensity 0 = background, intensity > 0 = foreground
    var color_idx: u32;
    if (intensity > 0u) {
        color_idx = fg_color;
    } else {
        color_idx = bg_color;
    }

    // Write to output
    let out_idx = py * params.canvas_width + px;
    output[out_idx] = color_idx;
}
