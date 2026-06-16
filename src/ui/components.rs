use std::collections::HashMap;

use log::log;
use ratatui::buffer::*;
use ratatui::layout::*;
use ratatui::style::*;
use ratatui::widgets::*;

use crate::ui::ui::*;

pub struct MonitorRender<'a> {
    pub scrapers: &'a HashMap<u64, UIScraper>,
}

pub struct ScraperRender<'a> {
    pub scraper: &'a UIScraper,
}

///
/// Overview for all scrapers and what they doing
///
impl<'a> Widget for MonitorRender<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut rows = Vec::new();
        let mut keys: Vec<&u64> = self.scrapers.keys().collect();
        keys.sort();

        for key in keys {
            if let Some(scraper) = self.scrapers.get(key) {
                let status_style = match scraper.status {
                    ScraperStatus::Idle => Style::default().fg(Color::DarkGray),
                    ScraperStatus::Running => Style::default().fg(Color::Green).bold(),
                    ScraperStatus::Completed => Style::default().fg(Color::Blue),
                    ScraperStatus::Failed => Style::default().fg(Color::Red),
                };

                let total_files = scraper.files.len();
                let files_summary = if total_files == 0 {
                    "No pending tasks".to_string()
                } else {
                    format!("{} files managed", total_files)
                };

                rows.push(Row::new(vec![
                    ratatui::widgets::Cell::new(scraper.worker.to_string()),
                    ratatui::widgets::Cell::new(scraper.name.clone()),
                    ratatui::widgets::Cell::new(format!("{:?}", scraper.status))
                        .style(status_style),
                    ratatui::widgets::Cell::new(files_summary),
                ]));
            }
        }

        let header = Row::new(vec!["ID", "Scraper Engine", "Status", "Payload Details"])
            .style(Style::default().fg(Color::Yellow).bold());

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(10),
                Constraint::Percentage(35),
                Constraint::Percentage(20),
                Constraint::Percentage(35),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Active Thread Monitor "),
        );

        ratatui::prelude::Widget::render(table, area, buf);
    }
}

///
/// View for the scraper individual files and logging.
///
impl<'a> Widget for ScraperRender<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // FIX 3: Rebuilt this layout to display deep info for ONE specific scraper screen
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Header panel summary card
        let summary_text = format!(
            "Worker ID: {}  |  Engine Profile: {}  |  Status: {:?}",
            self.scraper.worker, self.scraper.name, self.scraper.status
        );
        Paragraph::new(summary_text)
            .block(Block::default().borders(Borders::ALL).fg(Color::Cyan))
            .render(chunks[0], buf);

        log::info!("{:?}", self.scraper);
        // List out all the sub-file extraction workloads inside it
        let mut list_items = Vec::new();
        for (idx, file) in self.scraper.files.iter().enumerate() {
            let line = match file.status {
                FilesStatus::Waiting => {
                    ListItem::new(format!("  [{}] ⏳ File queued for processing...", idx))
                }
                FilesStatus::Downloading(progress) => ListItem::new(format!(
                    "  [{}] 📥 Downloading stream contents: {:.1}%",
                    idx, progress
                ))
                .fg(Color::Yellow),
                FilesStatus::Processing(progress) => ListItem::new(format!(
                    "  [{}] ⚙️ Evaluating extraction rules: {:.1}%",
                    idx, progress
                ))
                .fg(Color::LightBlue),
                FilesStatus::Done => ListItem::new(format!(
                    "  [{}] ✅ Processing execution completed successfully.",
                    idx
                ))
                .fg(Color::Green),
                FilesStatus::Stopped(ref stopped) => ListItem::new(format!(
                    "  [{}: {}] X Stopped execution completed successfully",
                    idx, stopped
                ))
                .fg(Color::Red),
            };
            list_items.push(line);
        }

        let list = List::new(list_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} Task Isolation Queue ", self.scraper.name)),
        );
        ratatui::prelude::Widget::render(list, chunks[1], buf);
    }
}
