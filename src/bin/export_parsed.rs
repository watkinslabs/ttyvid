use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process;

use ttygif_rust::terminal::{TerminalEmulator, parser::{parse_ansi_stream, Event, EscapeType}};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: export_parsed <cast_file> <output_file>");
        process::exit(1);
    }

    let cast_file = &args[1];
    let output_file = &args[2];

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

    // First, accumulate all output data (like Python does)
    let mut all_data = String::new();

    for line in lines {
        let line = line.expect("Failed to read line");
        if line.trim().is_empty() {
            continue;
        }

        let event: serde_json::Value = serde_json::from_str(&line).expect("Invalid event JSON");

        // Extract output data
        if event[1].as_str() == Some("o") {
            let data = event[2].as_str().expect("Invalid output data");
            all_data.push_str(data);
        }
    }

    // Now parse all data at once and export sequences
    let file = File::create(output_file).expect("Failed to create output file");
    let mut output = BufWriter::new(file);
    let mut seq_index = 0;

    let events = parse_ansi_stream(&all_data);

    for event in events {
        match event {
            Event::Text(chars) => {
                let text_len = chars.len();
                // Get preview (first 20 chars)
                let mut preview = String::new();
                for (i, &ch) in chars.iter().enumerate() {
                    if i >= 20 {
                        break;
                    }
                    let code = ch as u32;
                    if code < 32 || code >= 127 {
                        preview.push_str(&format!("\\x{:02x}", code));
                    } else {
                        preview.push(ch);
                    }
                }
                writeln!(output, "{:06} TEXT len={:5} preview=\"{}\"", seq_index, text_len, preview)
                    .expect("Failed to write");
            }
            Event::Command(cmd) => {
                let esc_type_str = match cmd.esc_type {
                    EscapeType::Single => "SINGLE",
                    EscapeType::CharSet => "CHAR_SET",
                    EscapeType::G0 => "G0",
                    EscapeType::G1 => "G1",
                    EscapeType::Csi => "CSI",
                    EscapeType::Osc => "OSC",
                    EscapeType::BracketPaste => "BRACKET_PA",
                    EscapeType::Title => "TITLE",
                };

                let param_str = cmd.params.iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(",");

                writeln!(output, "{:06} {:10} cmd={:5} params=[{}]",
                        seq_index, esc_type_str, cmd.command, param_str)
                    .expect("Failed to write");
            }
        }
        seq_index += 1;
    }

    // Also feed to terminal to match Python behavior
    terminal.feed_bytes(all_data.as_bytes());

    println!("Exported {} sequences to {}", seq_index, output_file);
}
