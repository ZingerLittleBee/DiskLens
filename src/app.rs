use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crossterm::event::Event;
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::config::settings::Settings;
use crate::core::events;
use crate::core::progress::ProgressTracker;
use crate::core::scanner::Scanner;
use crate::models::scan_result::ScanResult;
use crate::ui::app_state::AppState;
use crate::ui::input::{self, InputAction};
use crate::ui::renderer;

pub struct App {
    state: AppState,
    settings: Settings,
}

impl App {
    pub fn new(root_path: PathBuf, settings: Settings) -> Self {
        Self {
            state: AppState::new(root_path),
            settings,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Initialize terminal
        terminal::enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        // Start scan task
        let (event_tx, event_rx) = events::create_event_channel();
        let scanner = Scanner::new(self.settings.clone(), event_tx);
        let scan_path = self.state.current_path.clone();
        let progress = scanner.progress().clone();

        let scan_handle = tokio::spawn(async move { scanner.scan(scan_path).await });

        // Run main event loop
        let result = self.event_loop(&mut terminal, event_rx, &progress, scan_handle).await;

        // Restore terminal
        terminal::disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        mut event_rx: events::EventReceiver,
        progress: &Arc<ProgressTracker>,
        scan_handle: JoinHandle<anyhow::Result<ScanResult>>,
    ) -> anyhow::Result<()> {
        // Spawn a dedicated blocking thread for terminal input.
        // This sends crossterm events to the async world via an unbounded channel,
        // avoiding re-spawning spawn_blocking on every loop iteration.
        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Event>();
        let _input_thread = tokio::task::spawn_blocking(move || {
            loop {
                match input::poll_event(Duration::from_millis(50)) {
                    Ok(Some(event)) => {
                        if input_tx.send(event).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(_) => break,
                }
            }
        });

        let mut tick_interval = tokio::time::interval(Duration::from_millis(100));
        let mut scan_channel_open = true;
        // Wrap scan_handle in Option so we can take it once to await
        let mut scan_handle = Some(scan_handle);

        loop {
            // Render
            terminal.draw(|frame| {
                renderer::render(frame, &self.state);
            })?;

            tokio::select! {
                // Terminal input events
                input_event = input_rx.recv() => {
                    match input_event {
                        Some(Event::Key(key)) => {
                            let action = input::handle_key_event(key, &mut self.state);
                            match action {
                                InputAction::Quit => return Ok(()),
                                InputAction::Export => self.handle_export(),
                                _ => {}
                            }
                        }
                        Some(Event::Resize(_, _)) => {
                            // Terminal resized; next loop iteration will re-render
                        }
                        Some(_) => {}
                        None => return Ok(()),
                    }
                }
                // Scan events
                scan_event = event_rx.recv(), if scan_channel_open => {
                    match scan_event {
                        Some(events::Event::ScanCompleted { .. }) => {
                            // ScanCompleted is sent right before the scanner returns.
                            // The channel will close shortly after, and we collect
                            // the actual ScanResult from scan_handle below.
                        }
                        Some(events::Event::Progress { current_path, .. }) => {
                            let snapshot = progress.snapshot();
                            self.state.update_progress(
                                snapshot.files_scanned,
                                snapshot.total_size,
                                snapshot.files_per_second,
                                current_path.to_string_lossy().to_string(),
                            );
                            self.state.error_count = snapshot.errors_count;
                        }
                        Some(events::Event::ScanError { .. }) => {
                            let snapshot = progress.snapshot();
                            self.state.error_count = snapshot.errors_count;
                        }
                        Some(_) => {}
                        None => {
                            // Channel closed = scan finished (sender dropped).
                            scan_channel_open = false;
                        }
                    }
                }
                // Periodic tick for progress updates during scan
                _ = tick_interval.tick() => {
                    if self.state.scan_result.is_none() {
                        let snapshot = progress.snapshot();
                        self.state.update_progress(
                            snapshot.files_scanned,
                            snapshot.total_size,
                            snapshot.files_per_second,
                            self.state.current_scanning_path.clone(),
                        );
                        self.state.error_count = snapshot.errors_count;
                    }
                }
            }

            // When the scan event channel closes, collect the ScanResult
            if !scan_channel_open && self.state.scan_result.is_none() {
                if let Some(handle) = scan_handle.take() {
                    match handle.await {
                        Ok(Ok(result)) => self.state.set_scan_result(result),
                        Ok(Err(e)) => tracing::error!("Scan failed: {}", e),
                        Err(e) => tracing::error!("Scan task panicked: {}", e),
                    }
                }
            }

            if self.state.should_quit {
                return Ok(());
            }
        }
    }

    fn handle_export(&self) {
        if let Some(ref result) = self.state.scan_result {
            let path = PathBuf::from(format!(
                "disklens_report_{}.json",
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            ));
            if let Err(e) = crate::export::json::export_json(result, &path) {
                tracing::error!("Export failed: {}", e);
            } else {
                tracing::info!("Exported to: {}", path.display());
            }
        }
    }
}
