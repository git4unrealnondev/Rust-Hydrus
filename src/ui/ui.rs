use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use futures::StreamExt;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Widget},
};
use sharedtypes::HashesSupported;

use crate::ui::components::*;

use std::collections::HashMap;
use std::io;
pub struct App {
    exit: bool,
    receiver: tokio::sync::mpsc::UnboundedReceiver<UIEvent>,
    pub screen: Vec<AppScreen>,
    pub scrapers: HashMap<u64, UIScraper>,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum AppScreen {
    #[default]
    Monitor, // Main view flag
    ViewScraper(u64), // Carries the target worker ID we are actively viewing
}

#[derive(Debug, Clone)]
pub enum ScraperStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilesStatus {
    Waiting,
    Downloading(f64),
    Processing(f64),
    Done,
    Stopped(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileStorage {
    pub internal_id: u64,
    pub status: FilesStatus,
    pub hash: HashesSupported,
}

#[derive(Debug, Clone)]
pub enum UIEvent {
    /// Sent by the main worker when a scraper state shifts
    ScraperStatusChanged {
        worker_id: u64,
        name: String,
        status: ScraperStatus,
    },
    /// Sent by individual background download tasks to safely update status without overwriting anything else
    FileStatusChanged {
        worker_id: u64,
        job_id: u64,
        file_id: u64,
        status: FilesStatus,
    },
}

#[derive(Debug, Clone)]
pub struct UIScraper {
    pub worker: u64,
    pub name: String,
    pub status: ScraperStatus,
    pub files: HashMap<u64, Vec<FileStorage>>,
}

impl App {
    pub fn new(receiver: tokio::sync::mpsc::UnboundedReceiver<UIEvent>) -> Self {
        App {
            receiver,
            exit: false,
            screen: vec![AppScreen::Monitor],
            scrapers: HashMap::new(),
        }
    }

    pub async fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut reader = crossterm::event::EventStream::new();

        while !self.exit {
            // 1. DRAIN ALL PENDING EVENTS IN THE CHANNEL BUFFER IMMEDIATELY
            // This processes bursts of messages instantly without intermediate redraws
            while let Ok(event) = self.receiver.try_recv() {
                self.handle_ui_event_internal(event);
            }

            // 2. PAINT THE CURRENT STATE ONCE PER LOOP ROUND
            terminal.draw(|frame| self.draw(frame))?;

            // 3. CLEANLY BLOCK AND WAIT FOR THE NEXT NEW EVENT OR KEY PRESS
            tokio::select! {
                res = self.receiver.recv() => {
                    if let Some(event) = res {
                        self.handle_ui_event_internal(event);
                    } else {
                        // Optional: Senders dropped, handle if needed
                    }
                }
                Some(Ok(crossterm_event)) = reader.next() => {
                    match crossterm_event {
                        crossterm::event::Event::Key(key_event) => {
                            self.handle_key_event(key_event)?;
                        }
                        crossterm::event::Event::Resize(_, _) => {
                            // The loop will automatically clear and redraw on next pass
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    // Helper method to keep your event matching clean and reusable
    fn handle_ui_event_internal(&mut self, event: UIEvent) {
        match event {
            UIEvent::ScraperStatusChanged {
                worker_id,
                name,
                status,
            } => {
                let scraper = self.scrapers.entry(worker_id).or_insert_with(|| UIScraper {
                    worker: worker_id,
                    name: name.clone(),
                    status: status.clone(),
                    files: HashMap::new(),
                });
                scraper.name = name;
                scraper.status = status;
            }
            UIEvent::FileStatusChanged {
                worker_id,
                job_id,
                file_id,
                status,
            } => {
                // FIX: Ensure the scraper entry exists first instead of dropping the event!
                let scraper = self.scrapers.entry(worker_id).or_insert_with(|| UIScraper {
                    worker: worker_id,
                    name: "Initializing...".to_string(), // Will be updated by ScraperStatusChanged
                    status: ScraperStatus::Running,
                    files: HashMap::new(),
                });

                let file_vec = scraper.files.entry(job_id).or_insert_with(Vec::new);

                if let Some(file) = file_vec.iter_mut().find(|f| f.internal_id == file_id) {
                    file.status = status;
                } else {
                    file_vec.push(FileStorage {
                        internal_id: file_id,
                        status,
                        hash: sharedtypes::HashesSupported::None,
                    });
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(2), Constraint::Min(0)])
            .split(frame.area());

        let ribbon_text;
        if let Some(screen) = self.screen.last() {
            ribbon_text = match screen {
                AppScreen::Monitor => "⚡ [1-9] View Scraper Detail | [q] Quit",
                AppScreen::ViewScraper(_) => {
                    "⚡ [Esc/Backspace] Return to Monitor Panel | [q] Quit"
                }
            }
        } else {
            return;
        };

        let ribbon = Paragraph::new(ribbon_text).block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(ribbon, chunks[0]);

        frame.render_widget(self, chunks[1]);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<()> {
        if key_event.kind != KeyEventKind::Press {
            return Ok(());
        }

        match key_event.code {
            KeyCode::Esc => {
                let screen = self.screen.pop();
                if screen.is_none() || screen == Some(AppScreen::Monitor) {
                    self.exit = true;
                }
            }

            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Some(digit) = c.to_digit(10) {
                    let target_worker_id = (digit as u64);

                    // Only jump if that background thread has actually initialized and sent data!
                    if self.scrapers.contains_key(&target_worker_id) {
                        self.screen.push(AppScreen::ViewScraper(target_worker_id));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
// Handles drawing the main interaction body area
impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // FIX 1 & 2: Match against the self state reference safely
        if let Some(screen) = self.screen.last() {
            match screen {
                AppScreen::Monitor => {
                    // Pass a reference of the master hashmap to the renderer component
                    MonitorRender {
                        scrapers: &self.scrapers,
                    }
                    .render(area, buf);
                }
                AppScreen::ViewScraper(worker_id) => {
                    if let Some(scraper) = self.scrapers.get(worker_id) {
                        ScraperRender { scraper }.render(area, buf);
                    } else {
                        Paragraph::new(format!(
                            "Loading Scraper Thread (ID: {}) details...",
                            worker_id
                        ))
                        .render(area, buf);
                    }
                }
            }
        }
    }
}
