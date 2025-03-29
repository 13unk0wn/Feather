#![allow(unused)]
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::prelude::Alignment;
use ratatui::prelude::Buffer;
use ratatui::prelude::Constraint;
use ratatui::prelude::Direction;
use ratatui::prelude::Layout;
use ratatui::prelude::Rect;
use ratatui::prelude::Widget;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::backend::Backend;
use feather::PlaylistName;
use feather::config::USERCONFIG;

#[derive(PartialEq)]
enum SelectItem {
    YES,
    NO,
}

pub struct DeleteUserPlaylistPopUp {
    state: SelectItem,
    config: Rc<USERCONFIG>,
    backend: Arc<Backend>,
    pub playlist_name: Option<String>,
}

impl DeleteUserPlaylistPopUp {
    pub fn new(config: Rc<USERCONFIG>, backend: Arc<Backend>) -> Self {
        Self {
            state: SelectItem::NO,
            config,
            backend,
            playlist_name: None,
        }
    }

    fn change_state(&mut self) {
        match self.state {
            SelectItem::YES => self.state = SelectItem::NO,
            SelectItem::NO => self.state = SelectItem::YES,
        }
    }

    pub fn handle_keystokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => self.change_state(),
            KeyCode::Enter => {
                if self.state == SelectItem::YES {
                    if let Some(playlist_name) = &self.playlist_name {
                        self.backend.PlayListManager.delete_playlist(playlist_name);
                    }
                }
                self.playlist_name = None;
            }
            _ => (),
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Clear the area before rendering
        Clear.render(area, buf);

        let bg_color = self.config.bg_color;
        let text_color = self.config.text_color;
        let global_style = Style::default()
            .fg(Color::Rgb(text_color.0, text_color.1, text_color.2))
            .bg(Color::Rgb(bg_color.0, bg_color.1, bg_color.2));

        // Render background block
        Block::default().style(global_style).render(area, buf);

        // Create a layout for the confirmation UI
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Top empty space
                Constraint::Min(5),         // Dialog box
                Constraint::Percentage(40), // Bottom empty space
            ])
            .split(area);

        let popup_area = chunks[1];

        // Render confirmation box
        let confirmation_block = Block::default()
            .title("Confirmation")
            .title_bottom("[TAB] - toggle")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .style(global_style);

        confirmation_block.render(popup_area, buf);

        // Center text inside the confirmation block
        let text_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Question
                Constraint::Length(1), // Empty space
                Constraint::Length(1), // YES/NO options
            ])
            .split(popup_area);

        let question = Paragraph::new("Do you want to delete the playlist?")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White));

        let color = self.config.selected_mode_text_color;
        // Set styles for YES and NO options
        let yes_style = if matches!(self.state, SelectItem::YES) {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(color.0, color.1, color.2)) // Highlight YES
        } else {
            Style::default().fg(Color::Yellow).bg(Color::Reset)
        };

        let no_style = if matches!(self.state, SelectItem::NO) {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(color.0, color.1, color.2)) // Highlight YES
        } else {
            Style::default().fg(Color::Yellow).bg(Color::Reset)
        };

        let span = Line::from(vec![
            Span::styled(" [ YES ] ", yes_style),
            Span::raw("   "), // Spacer
            Span::styled(" [ NO ] ", no_style),
        ]);

        let options = Paragraph::new(span).alignment(Alignment::Center);

        question.render(text_area[0], buf);
        options.render(text_area[2], buf);
    }
}
