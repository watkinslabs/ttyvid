use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use super::{CaptureSession, CastWriter, RecordConfig};

/// PTY-based recorder that handles all terminal I/O
pub struct PtyRecorder {
    master: Box<dyn MasterPty + Send>,
    config: RecordConfig,
}

impl PtyRecorder {
    pub fn new(config: &RecordConfig) -> Result<Self> {
        let pty_system = native_pty_system();

        // Create PTY with specified dimensions
        let pty_size = PtySize {
            rows: config.rows,
            cols: config.columns,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(pty_size)
            .context("Failed to open PTY")?;

        // Build command
        let mut cmd = if let Some(ref command) = config.command {
            // Execute specific command
            let mut builder = CommandBuilder::new(&command[0]);
            if command.len() > 1 {
                builder.args(&command[1..]);
            }
            builder
        } else {
            // Spawn user's shell - cross-platform defaults
            let shell = std::env::var("SHELL").unwrap_or_else(|_| {
                #[cfg(windows)]
                {
                    // On Windows, use cmd.exe or PowerShell
                    std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
                }
                #[cfg(not(windows))]
                {
                    // On Unix-like systems, default to sh (always available)
                    "/bin/sh".to_string()
                }
            });
            CommandBuilder::new(shell)
        };

        // Set current working directory to where ttyvid was invoked
        if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Set TERM environment variable
        cmd.env("TERM", "xterm-256color");

        // Spawn the child process in the PTY slave
        let _child = pair
            .slave
            .spawn_command(cmd)
            .context("Failed to spawn command in PTY")?;

        // We only need the master side - slave is owned by child process
        Ok(Self {
            master: pair.master,
            config: config.clone(),
        })
    }

    /// Run the recording session with I/O multiplexing
    pub fn run(&mut self, capture: &mut CaptureSession, writer: &mut CastWriter) -> Result<()> {
        // Channel for stdin thread to communicate
        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();

        // Check if stdin is a TTY
        let is_tty = crossterm::tty::IsTty::is_tty(&std::io::stdin());

        // Spawn thread to read user input from stdin (only if TTY)
        let stdin_thread = if is_tty {
            Some(thread::spawn(move || {
                Self::stdin_reader_thread(tx)
            }))
        } else {
            None
        };

        // Put our terminal in raw mode so we can forward keystrokes (only if TTY)
        if is_tty {
            enable_raw_mode().context("Failed to enable raw mode")?;
        }

        let result = self.io_loop(capture, writer, rx, is_tty);

        // Restore terminal (only if TTY)
        if is_tty {
            disable_raw_mode().context("Failed to disable raw mode")?;
        }

        // Don't wait for stdin thread - it's blocking on read and will exit naturally
        // Waiting for it would require the user to press a key
        drop(stdin_thread);

        result
    }

    /// Main I/O loop - multiplex between PTY output and stdin input
    fn io_loop(
        &mut self,
        capture: &mut CaptureSession,
        writer: &mut CastWriter,
        stdin_rx: Receiver<Vec<u8>>,
        is_tty: bool,
    ) -> Result<()> {
        let reader = self.master.try_clone_reader().context("Failed to clone PTY reader")?;
        let mut pty_writer = self.master.take_writer().context("Failed to get PTY writer")?;

        // Create channel for PTY output
        let (pty_tx, pty_rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();

        // Spawn thread to read from PTY (blocking in separate thread is fine)
        let pty_thread = thread::spawn(move || {
            Self::pty_reader_thread(reader, pty_tx, is_tty)
        });

        let mut last_stats_update = std::time::Instant::now();

        loop {
            // Check for PTY output
            match pty_rx.try_recv() {
                Ok(data) => {
                    if data.is_empty() {
                        // EOF signal - child process exited
                        break;
                    }

                    // Capture the output with timestamp
                    let event = capture.record_output(&data);

                    // Write to .cast file
                    writer.write_event(&event)?;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // No data available yet
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // PTY reader thread ended
                    break;
                }
            }

            // Check for user input from stdin - drain all available input
            let mut has_input = false;
            while let Ok(input) = stdin_rx.try_recv() {
                // Forward to PTY immediately
                pty_writer.write_all(&input)?;
                has_input = true;
            }
            if has_input {
                pty_writer.flush()?;
            }

            // Update stats if enabled
            if self.config.show_stats && last_stats_update.elapsed() > Duration::from_millis(500) {
                let stats = capture.get_stats();
                eprint!("\r\x1b[K"); // Clear line
                eprint!("Recording: {}s | {} events", stats.duration_secs, stats.event_count);
                last_stats_update = std::time::Instant::now();
            }

            // Small sleep to avoid busy-waiting
            thread::sleep(Duration::from_millis(10));
        }

        // Wait for PTY reader thread
        let _ = pty_thread.join();

        if self.config.show_stats {
            eprint!("\r\x1b[K"); // Clear stats line
        }

        Ok(())
    }

    /// Thread function to read from stdin
    fn stdin_reader_thread(tx: Sender<Vec<u8>>) {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 1024];

        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break; // Main thread disconnected
                    }
                }
                Err(_) => break,
            }
        }
    }

    /// Thread function to read from PTY
    fn pty_reader_thread(mut reader: Box<dyn Read + Send>, tx: Sender<Vec<u8>>, is_tty: bool) {
        let mut buf = [0u8; 8192];

        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    // EOF - send empty vec as signal
                    let _ = tx.send(Vec::new());
                    break;
                }
                Ok(n) => {
                    let data = buf[..n].to_vec();

                    // Write to stdout if TTY (so user sees output in real-time)
                    if is_tty {
                        let _ = std::io::stdout().write_all(&data);
                        let _ = std::io::stdout().flush();
                    }

                    // Send to main thread for recording
                    if tx.send(data).is_err() {
                        break; // Main thread disconnected
                    }
                }
                Err(_) => break,
            }
        }
    }
}

impl Drop for PtyRecorder {
    fn drop(&mut self) {
        // Ensure raw mode is disabled
        let _ = disable_raw_mode();
    }
}
