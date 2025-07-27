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
    event::{AppEvent, EventHandler},
    state::AppState,
    ui::draw_ui,
};
use std::sync::Arc;

pub struct App {
    state: AppState,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    event_handler: EventHandler,
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
        let event_handler = EventHandler::new();

        Ok(Self {
            state,
            terminal,
            event_handler,
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
        // Global key bindings
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                return Ok(true); // Quit
            }
            KeyCode::Esc => {
                if self.state.input_mode {
                    self.state.input_mode = false;
                    self.state.input_buffer.clear();
                } else {
                    return Ok(true); // Quit
                }
            }
            _ => {}
        }

        // Handle events based on current state
        let event = self.event_handler.handle_key_event(key, &mut self.state)?;
        if let Some(app_event) = event {
            self.handle_app_event(app_event).await?;
        }

        Ok(false)
    }

    async fn handle_app_event(&mut self, event: AppEvent) -> Result<(), TermComError> {
        match event {
            AppEvent::Quit => {
                self.should_quit = true;
            }
            AppEvent::SwitchPanel(panel) => {
                self.state.active_panel = panel;
                self.state.input_mode = false;
            }
            AppEvent::ToggleInputMode => {
                self.state.input_mode = !self.state.input_mode;
                if !self.state.input_mode {
                    self.state.input_buffer.clear();
                }
            }
            AppEvent::SendMessage(message) => {
                if let Some(session_id) = &self.state.selected_session {
                    self.state.add_message(session_id.clone(), message, true).await?;
                }
            }
            AppEvent::ConnectSerial { port, baud_rate } => {
                self.state.create_serial_session(&self.session_manager, port, baud_rate).await?;
            }
            AppEvent::ConnectTcp { host, port } => {
                self.state.create_tcp_session(&self.session_manager, host, port).await?;
            }
            AppEvent::SelectSession(session_id) => {
                self.state.selected_session = Some(session_id);
            }
            AppEvent::CloseSession(session_id) => {
                self.state.close_session(session_id).await?;
            }
        }
        Ok(())
    }

    async fn tick(&mut self) -> Result<(), TermComError> {
        // Update session status and receive messages
        self.state.update_sessions().await?;
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