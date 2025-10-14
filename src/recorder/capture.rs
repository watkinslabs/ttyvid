use chrono::Utc;
use std::time::Instant;
use super::RecordConfig;

/// Represents a single output event with timestamp
#[derive(Debug, Clone)]
pub struct OutputEvent {
    /// Time offset from start of recording (seconds)
    pub timestamp: f64,

    /// Output data (raw bytes)
    pub data: Vec<u8>,
}

/// Statistics about the recording session
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub duration_secs: u64,
    pub event_count: usize,
    pub bytes_recorded: usize,
}

/// Manages capture timing and event recording
pub struct CaptureSession {
    /// Start time of recording
    start_time: Instant,

    /// Last event timestamp
    last_timestamp: f64,

    /// Configuration
    config: RecordConfig,

    /// Total events recorded
    event_count: usize,

    /// Total bytes recorded
    bytes_recorded: usize,

    /// Paused state
    paused: bool,

    /// Time spent paused
    pause_start: Option<Instant>,
    total_pause_duration: f64,
}

impl CaptureSession {
    pub fn new(config: &RecordConfig) -> Self {
        Self {
            start_time: Instant::now(),
            last_timestamp: 0.0,
            config: config.clone(),
            event_count: 0,
            bytes_recorded: 0,
            paused: false,
            pause_start: None,
            total_pause_duration: 0.0,
        }
    }

    /// Record output data and return the event
    pub fn record_output(&mut self, data: &[u8]) -> OutputEvent {
        if self.paused {
            // Don't record while paused
            // (In a full implementation, we'd buffer and emit when unpaused)
            return OutputEvent {
                timestamp: self.last_timestamp,
                data: Vec::new(),
            };
        }

        // Calculate timestamp (elapsed time minus pause duration)
        let elapsed = self.start_time.elapsed().as_secs_f64() - self.total_pause_duration;

        // Apply max_idle compression if configured
        let timestamp = if let Some(max_idle) = self.config.max_idle {
            let idle_time = elapsed - self.last_timestamp;
            if idle_time > max_idle {
                // Compress idle time to max_idle
                self.last_timestamp + max_idle
            } else {
                elapsed
            }
        } else {
            elapsed
        };

        self.last_timestamp = timestamp;
        self.event_count += 1;
        self.bytes_recorded += data.len();

        OutputEvent {
            timestamp,
            data: data.to_vec(),
        }
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        if self.paused {
            // Unpause
            if let Some(pause_start) = self.pause_start {
                self.total_pause_duration += pause_start.elapsed().as_secs_f64();
            }
            self.paused = false;
            self.pause_start = None;
        } else {
            // Pause
            self.paused = true;
            self.pause_start = Some(Instant::now());
        }
    }

    /// Get current session statistics
    pub fn get_stats(&self) -> SessionStats {
        let duration = self.start_time.elapsed().as_secs();

        SessionStats {
            duration_secs: duration,
            event_count: self.event_count,
            bytes_recorded: self.bytes_recorded,
        }
    }

    /// Get start timestamp for header
    pub fn get_start_timestamp() -> i64 {
        Utc::now().timestamp()
    }
}
