use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use crate::{
    domain::error::TermComError,
    core::{
        communication::CommunicationEngine,
        session::SessionManager,
    },
};
use super::{
    state::AppState,
    ui::{draw_ui, ViewMode},
};
use std::sync::Arc;

pub struct App {
    state: AppState,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    should_quit: bool,
    last_tick: Instant,
    tick_rate: Duration,
    session_manager: Arc<SessionManager>,
    communication_engine: Arc<CommunicationEngine>,
}

impl App {
    pub fn new() -> Result<Self, TermComError> {
        // Setup terminal
        enable_raw_mode().map_err(|e| TermComError::TuiError(e.to_string()))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| TermComError::TuiError(e.to_string()))?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| TermComError::TuiError(e.to_string()))?;

        // Initialize communication engine and session manager
        let communication_engine = Arc::new(CommunicationEngine::new(1000, 10));
        let session_manager = Arc::new(SessionManager::new(Arc::clone(&communication_engine), 10));

        let state = AppState::new();

        Ok(Self {
            state,
            terminal,
            should_quit: false,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(250),
            session_manager,
            communication_engine,
        })
    }

    pub async fn run(&mut self) -> Result<(), TermComError> {
        // Start the communication engine
        self.communication_engine.start().await?;
        
        loop {
            // Handle events
            if let Ok(true) = event::poll(self.tick_rate) {
                if let Ok(event) = event::read() {
                    match event {
                        Event::Key(key) => {
                            if self.handle_key_event(key).await? {
                                break;
                            }
                        }
                        Event::Resize(width, height) => {
                            self.state.terminal_size = (width, height);
                        }
                        _ => {}
                    }
                }
            }

            // Tick
            if self.last_tick.elapsed() >= self.tick_rate {
                self.tick().await?;
                self.last_tick = Instant::now();
            }

            // Draw UI
            self.terminal
                .draw(|f| draw_ui(f, &mut self.state))
                .map_err(|e| TermComError::TuiError(e.to_string()))?;

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    async fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<bool, TermComError> {
        // Input mode handling
        if self.state.input_mode {
            match key.code {
                KeyCode::Enter => {
                    let input = self.state.input_buffer.clone();
                    self.state.input_buffer.clear();
                    self.state.input_mode = false;
                    
                    match self.state.view_mode {
                        ViewMode::Chat => {
                            if self.state.get_connection().is_some() {
                                self.state.add_message(input, true).await?;
                            }
                        }
                        ViewMode::Command => {
                            self.handle_command(input).await?;
                        }
                    }
                }
                KeyCode::Esc => {
                    self.state.input_mode = false;
                    self.state.input_buffer.clear();
                }
                KeyCode::Backspace => {
                    self.state.input_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.state.input_buffer.push(c);
                }
                _ => {}
            }
            return Ok(false);
        }

        // Normal mode key bindings
        match key.code {
            KeyCode::Char('q') => return Ok(true), // Quit
            KeyCode::Esc => return Ok(true), // Quit
            KeyCode::Char('h') => {
                self.state.toggle_help();
            }
            KeyCode::Tab => {
                self.state.view_mode = match self.state.view_mode {
                    ViewMode::Chat => ViewMode::Command,
                    ViewMode::Command => ViewMode::Chat,
                };
            }
            KeyCode::Char('i') => {
                if matches!(self.state.view_mode, ViewMode::Chat) {
                    self.state.input_mode = true;
                }
            }
            KeyCode::Char(':') => {
                self.state.view_mode = ViewMode::Command;
                self.state.input_mode = true;
            }
            _ => {}
        }

        Ok(false)
    }

    async fn handle_command(&mut self, command: String) -> Result<(), TermComError> {
        let command = command.trim();
        if !command.starts_with(':') {
            self.state.set_status_message("Commands must start with ':'".to_string());
            return Ok(());
        }
        
        let command = &command[1..]; // Remove ':'
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        match parts.get(0) {
            Some(&"serial") => {
                if parts.len() >= 3 {
                    let port = parts[1].to_string();
                    if let Ok(baud_rate) = parts[2].parse::<u32>() {
                        self.state.create_serial_connection(&self.session_manager, port, baud_rate).await?;
                        self.state.view_mode = ViewMode::Chat;
                    } else {
                        self.state.set_status_message("Invalid baud rate".to_string());
                    }
                } else {
                    self.state.set_status_message("Usage: :serial <port> <baud_rate>".to_string());
                }
            }
            Some(&"tcp") => {
                if parts.len() >= 3 {
                    let host = parts[1].to_string();
                    if let Ok(port) = parts[2].parse::<u16>() {
                        self.state.create_tcp_connection(&self.session_manager, host, port).await?;
                        self.state.view_mode = ViewMode::Chat;
                    } else {
                        self.state.set_status_message("Invalid port number".to_string());
                    }
                } else {
                    self.state.set_status_message("Usage: :tcp <host> <port>".to_string());
                }
            }
            Some(&"close") => {
                self.state.close_connection().await?;
            }
            Some(&"quit") => {
                self.should_quit = true;
            }
            Some(cmd) => {
                self.state.set_status_message(format!("Unknown command: {}", cmd));
            }
            None => {
                self.state.set_status_message("Empty command".to_string());
            }
        }
        
        Ok(())
    }

    async fn tick(&mut self) -> Result<(), TermComError> {
        // Update connection status and receive messages
        self.state.update_connection().await?;
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}