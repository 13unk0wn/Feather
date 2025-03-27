#![allow(unused)]
use crate::backend::Backend;
use crossterm::event::KeyEvent;
use ratatui::prelude::Widget;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
enum HomeErorr {
    #[error("Image not Found : {0}")]
    ImageNotFound(String),
}

pub struct Home {
    backend: Arc<Backend>,
}

impl Home {
    pub fn new(backend: Arc<Backend>) -> Self {
        Self { backend }
    }

    pub fn handle_keywords(&self, key: KeyEvent) {}

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(area);

        let image_area = chunks[0];
        let stats_area = chunks[1];

        let get_data = self.backend.user_profile.give_info().unwrap();

        let user_stats = vec![
            Line::from(vec![
                Span::styled(
                    "👤 User: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(get_data.name, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "🎵 Last Played: ",
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    get_data
                        .last_played
                        .as_ref()
                        .map(|s| s.title.clone())
                        .unwrap_or("None".to_string()),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "📀 Songs Played: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    get_data.songs_played.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "⏳ Time Played: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} secs", get_data.time_played / 60),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        let stats_block = Block::default()
            .borders(Borders::ALL)
            .title(" 🎼 USER STATS ")
            .title_alignment(Alignment::Center)
            .border_style(Style::default().fg(Color::LightBlue));

        let paragraph = Paragraph::new(user_stats)
            .block(stats_block)
            .alignment(Alignment::Left);

        paragraph.render(stats_area, buf);
    }
}
