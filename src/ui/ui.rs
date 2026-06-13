use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    // 1. Removed 'Padding' from Ratatui imports to avoid ambiguity
    widgets::{Block, Borders, Paragraph, Widget},
};
use ratatui_garnish::{
    GarnishableWidget,
    Padding, // 2. Kept the Garnish padding explicitly
    border::RoundedDashedBorder,
    garnishes,
    title::{Above, Title},
};
use std::io;
pub struct App {
    exit: bool,
}

impl App {
    pub fn new() -> Self {
        App { exit: false }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            // Draw before blocking on input so the screen updates immediately
            terminal.draw(|frame| self.draw(frame))?;

            match crossterm::event::read()? {
                crossterm::event::Event::Key(key_event) => self.handle_key_event(key_event)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        // Create a 2-part vertical split: Top ribbon and Bottom main area
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Max(2), Constraint::Min(0)])
            .split(frame.area());

        // 1. Render the top ribbon actions
        let ribbon = Paragraph::new("⚡ [F1] Start  |  [F2] Stop  |  [F3] Pause  |  [q] Quit");
        frame.render_widget(ribbon, chunks[0]);

        // 2. Render the main workspace (delegated to the App widget implementation)
        frame.render_widget(self, chunks[1]);
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
            self.exit = true;
        }
        Ok(())
    }
}

// Handles drawing the main interaction body area
impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        // 1. Build a multi-line Text object using Ratatui's raw or vector constructors
        let text_data = Text::raw("Hello, World!\nTasty TUIs from Ratatui");

        // 2. Call .garnish directly on the owned Text instance.
        // This completely bypasses the Paragraph reference trait restriction.
        let widget = &text_data
            .garnish(RoundedDashedBorder::default()) // Add your rounded dashed border
            .garnish(Title::<Above>::raw("My App")) // Add a title above
            .garnish(Style::default().bg(Color::Black)) // Set your background container color
            .garnish(Padding::uniform(1)); // Provide inside container padding

        // 3. Render it directly to the local buffer grid
        widget.render(area, buf);
    }
}
