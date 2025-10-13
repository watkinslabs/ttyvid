use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref ANSI_REGEX: Regex = {
        // Patterns copied EXACTLY from Python parser.pyx lines 295-304
        // ANSI_SINGLE   = '[\033]([cDEHMZ6789>=i])'
        // ANSI_CHAR_SET = '[\033]\\%([@G*])'
        // ANSI_G0       = '[\033]\\(([B0UK])'
        // ANSI_G1       = '[\033]\\)([B0UK])'
        // ANSI_CSI_RE   = '[\033]\\[((?:\\d|;|<|>|=|\?)*)([a-zA-Z])\002?'
        // ANSI_OSC      = '(?:\033\\]|\x9d).*?(?:\033\\\\|[\a\x9c])'
        // BRACKET_PASTE = '[\033]\\[(20[0-1]~)'
        // ANSI_TITLE    = '[\033][k](.*)[\033][\\\\]'

        let pattern = concat!(
            r"([\x1b]([cDEHMZ6789>=i]))",
            "|",
            r"([\x1b]\\%([@G*]))",
            "|",
            r"([\x1b]\(([B0UK]))",
            "|",
            r"([\x1b]\)([B0UK]))",
            "|",
            r"([\x1b]\[((?:\d|;|<|>|=|\?)*)([a-zA-Z`~])\x02?)",
            "|",
            r"((?:[\x1b]\]|\x9d).*?(?:[\x1b]\\|[\x07\x9c]))",
            "|",
            r"([\x1b]\[(20[0-1])~)",
            "|",
            r"([\x1b][k](.*?)[\x1b]\\)"
        );
        Regex::new(pattern).unwrap()
    };
}

#[derive(Debug, Clone)]
pub enum EscapeType {
    Single,
    CharSet,
    G0,
    G1,
    Csi,
    Osc,
    BracketPaste,
    Title,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub esc_type: EscapeType,
    pub command: String,
    pub params: Vec<i32>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Text(Vec<char>),
    Command(Command),
}

pub fn parse_ansi_stream(text: &str) -> Vec<Event> {
    let (events, _) = parse_ansi_stream_with_position(text);
    events
}

// Exact translation of Python parser.pyx stream_2_sequence lines 284-361
// Returns events and the last position parsed (cursor in Python)
pub fn parse_ansi_stream_with_position(text: &str) -> (Vec<Event>, usize) {
    let mut events = Vec::new();
    let mut last_pos = 0;

    for cap in ANSI_REGEX.captures_iter(text) {
        let match_start = cap.get(0).unwrap().start();
        let match_end = cap.get(0).unwrap().end();

        // Add text before this match (lines 287-288)
        if match_start > last_pos {
            let text_slice = &text[last_pos..match_start];
            if !text_slice.is_empty() {
                let chars: Vec<char> = text_slice.chars().collect();
                events.push(Event::Text(chars));
            }
        }

        // Parse the escape sequence (line 80)
        if let Some(event) = parse_escape_sequence(&cap) {
            events.push(event);
        }

        last_pos = match_end;  // line 289: cursor = end
    }

    // Return events and cursor position
    // Note: remaining text is NOT added to events, matching Python lines 355-360
    (events, last_pos)
}

fn parse_escape_sequence(cap: &regex::Captures) -> Option<Event> {
    // SINGLE (group 1, 2)
    if cap.get(1).is_some() {
        if let Some(cmd) = cap.get(2) {
            return Some(Event::Command(Command {
                esc_type: EscapeType::Single,
                command: cmd.as_str().to_string(),
                params: vec![],
            }));
        }
    }

    // CHAR_SET (group 3, 4)
    if cap.get(3).is_some() {
        if let Some(cmd) = cap.get(4) {
            return Some(Event::Command(Command {
                esc_type: EscapeType::CharSet,
                command: cmd.as_str().to_string(),
                params: vec![],
            }));
        }
    }

    // G0 (group 5, 6)
    if cap.get(5).is_some() {
        if let Some(cmd) = cap.get(6) {
            return Some(Event::Command(Command {
                esc_type: EscapeType::G0,
                command: cmd.as_str().to_string(),
                params: vec![],
            }));
        }
    }

    // G1 (group 7, 8)
    if cap.get(7).is_some() {
        if let Some(cmd) = cap.get(8) {
            return Some(Event::Command(Command {
                esc_type: EscapeType::G1,
                command: cmd.as_str().to_string(),
                params: vec![],
            }));
        }
    }

    // CSI (group 9, 10, 11)
    if cap.get(9).is_some() {
        let param_str = cap.get(10).map(|m| m.as_str()).unwrap_or("");
        let command = cap.get(11).map(|m| m.as_str()).unwrap_or("");

        let (cmd_str, params) = parse_csi_params(param_str, command);

        return Some(Event::Command(Command {
            esc_type: EscapeType::Csi,
            command: cmd_str,
            params,
        }));
    }

    // OSC (group 12)
    if cap.get(12).is_some() {
        return Some(Event::Command(Command {
            esc_type: EscapeType::Osc,
            command: String::new(),
            params: vec![],
        }));
    }

    // BRACKET_PASTE (group 13, 14)
    if cap.get(13).is_some() {
        if let Some(code) = cap.get(14) {
            let value = code.as_str().parse().unwrap_or(0);
            return Some(Event::Command(Command {
                esc_type: EscapeType::BracketPaste,
                command: "~".to_string(),
                params: vec![value],
            }));
        }
    }

    // TITLE (group 15, 16)
    if cap.get(15).is_some() {
        return Some(Event::Command(Command {
            esc_type: EscapeType::Title,
            command: "0".to_string(),
            params: vec![],
        }));
    }

    None
}

fn parse_csi_params(param_str: &str, command: &str) -> (String, Vec<i32>) {
    // Handle H and f specially (cursor position)
    if command == "H" || command == "f" {
        let parts: Vec<&str> = param_str.split(';').collect();
        let mut params = vec![];
        for part in parts {
            params.push(if part.is_empty() { 1 } else { part.parse().unwrap_or(1) });
        }
        while params.len() < 2 {
            params.push(1);
        }
        return (command.to_string(), params);
    }

    // Handle DEC Private Mode (DECSET/DECRST) sequences
    if !param_str.is_empty() && param_str.starts_with('?') {
        let cmd_str = format!("?{}", command);
        let param_tokens: Vec<&str> = param_str[1..].split(';').collect();
        let params: Vec<i32> = param_tokens.iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        return (cmd_str, params);
    }

    // Normal CSI parameters
    let params: Vec<i32> = param_str.split(';')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    // Default parameters for certain commands
    let params = if params.is_empty() {
        if command == "J" || command == "K" || command == "m" {
            vec![0]
        } else if command == "A" || command == "B" || command == "C" || command == "D" {
            vec![1]
        } else {
            vec![]
        }
    } else {
        params
    };

    (command.to_string(), params)
}
