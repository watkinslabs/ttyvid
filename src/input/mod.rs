use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Event {
    pub timestamp: f64,
    pub event_type: EventType,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Output,
    Input,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub width: usize,
    pub height: usize,
    pub title: Option<String>,
}

pub trait InputSource {
    fn read_events(&mut self) -> Result<Vec<Event>>;
    fn metadata(&self) -> Metadata;
}

// Asciicast v2 format structures
#[derive(Debug, Deserialize)]
struct AsciicastHeader {
    version: u32,
    width: usize,
    height: usize,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AsciicastEvent(f64, String, String);

pub struct AsciicastReader {
    header: AsciicastHeader,
    events: Vec<Event>,
}

impl AsciicastReader {
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Parse header (first line)
        let header_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty asciicast file"))??;
        let header: AsciicastHeader = serde_json::from_str(&header_line)?;

        // Parse events
        let mut events = Vec::new();
        for line in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let event: AsciicastEvent = serde_json::from_str(&line)?;
            events.push(Event {
                timestamp: event.0,
                event_type: match event.1.as_str() {
                    "o" => EventType::Output,
                    "i" => EventType::Input,
                    _ => EventType::Output,
                },
                data: event.2.as_bytes().to_vec(),
            });
        }

        Ok(Self { header, events })
    }
}

impl InputSource for AsciicastReader {
    fn read_events(&mut self) -> Result<Vec<Event>> {
        Ok(self.events.clone())
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            width: self.header.width,
            height: self.header.height,
            title: self.header.title.clone(),
        }
    }
}

// Stdin reader - supports both asciicast format and raw terminal data
pub struct StdinReader {
    width: usize,
    height: usize,
    metadata: Option<Metadata>,
    events: Option<Vec<Event>>,
}

impl StdinReader {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            metadata: None,
            events: None,
        }
    }

    fn parse_stdin(&mut self) -> Result<()> {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;

        // Try to parse as asciicast format first
        if let Ok((meta, events)) = Self::try_parse_asciicast(&buffer) {
            self.metadata = Some(meta);
            self.events = Some(events);
        } else {
            // Fall back to treating as raw terminal data
            self.metadata = Some(Metadata {
                width: self.width,
                height: self.height,
                title: None,
            });
            self.events = Some(vec![Event {
                timestamp: 0.0,
                event_type: EventType::Output,
                data: buffer.as_bytes().to_vec(),
            }]);
        }

        Ok(())
    }

    fn try_parse_asciicast(content: &str) -> Result<(Metadata, Vec<Event>)> {
        let mut lines = content.lines();

        // Parse header (first line)
        let header_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty input"))?;
        let header: AsciicastHeader = serde_json::from_str(header_line)?;

        // Parse events
        let mut events = Vec::new();
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let event: AsciicastEvent = serde_json::from_str(line)?;
            events.push(Event {
                timestamp: event.0,
                event_type: match event.1.as_str() {
                    "o" => EventType::Output,
                    "i" => EventType::Input,
                    _ => EventType::Output,
                },
                data: event.2.as_bytes().to_vec(),
            });
        }

        let metadata = Metadata {
            width: header.width,
            height: header.height,
            title: header.title,
        };

        Ok((metadata, events))
    }
}

impl InputSource for StdinReader {
    fn read_events(&mut self) -> Result<Vec<Event>> {
        if self.events.is_none() {
            self.parse_stdin()?;
        }
        Ok(self.events.as_ref().unwrap().clone())
    }

    fn metadata(&self) -> Metadata {
        self.metadata.as_ref().map(|m| m.clone()).unwrap_or_else(|| Metadata {
            width: self.width,
            height: self.height,
            title: None,
        })
    }
}
