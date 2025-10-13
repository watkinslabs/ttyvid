use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process;

use ttyvid::terminal::TerminalEmulator;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: dump_grid <cast_file>");
        process::exit(1);
    }

    let cast_file = &args[1];

    // Read the cast file
    let file = File::open(cast_file).expect("Failed to open cast file");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Skip header
    let first_line = lines.next().expect("Empty file").expect("Failed to read header");
    let header: serde_json::Value = serde_json::from_str(&first_line).expect("Invalid header JSON");

    let width = header["width"].as_u64().unwrap_or(80) as usize;
    let height = header["height"].as_u64().unwrap_or(24) as usize;

    // Create terminal emulator with default colors
    let mut terminal = TerminalEmulator::new(width, height, true, 7, 0);

    // Process all events
    for line in lines {
        let line = line.expect("Failed to read line");
        if line.trim().is_empty() {
            continue;
        }

        let event: serde_json::Value = serde_json::from_str(&line).expect("Invalid event JSON");

        // Extract output data
        if event[1].as_str() == Some("o") {
            let data = event[2].as_str().expect("Invalid output data");
            terminal.feed_bytes(data.as_bytes());
        }
    }

    // Dump the terminal grid
    let grid = terminal.grid();

    println!("Terminal Grid ({}x{}):", width, height);
    println!("================================================================================");

    for y in 0..grid.height() {
        print!("{:2} |", y);
        for x in 0..grid.width() {
            if let Some(cell) = grid.get_cell(x, y) {
                let ch = cell.character;
                if ch == '\0' || ch == ' ' {
                    print!(".");
                } else if (ch as u32) < 32 {
                    print!("?");
                } else {
                    print!("{}", ch);
                }
            } else {
                print!(".");
            }
        }
        println!("|");
    }
    println!("================================================================================");

    // Dump cursor position
    let state = terminal.state();
    println!("Cursor: ({}, {})", state.cursor_x, state.cursor_y);
}
