#![allow(unused)]
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::PlaylistName;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Widget;
use std::collections::linked_list;
use std::sync::Arc;
use tui_textarea::TextArea;

use crate::backend::Backend;

enum State {
    AllPlayList,
    CreatePlayList,
    ViewPlayList,
}

struct UserPlayList<'a> {
    backend: Arc<Backend>,
    state: State,
    new_playlist: NewPlayList<'a>,
}

impl<'a> UserPlayList<'a> {
    fn new(backend: Arc<Backend>) -> Self {
        Self {
            backend: backend.clone(),
            state: State::AllPlayList,
            new_playlist: NewPlayList::new(backend),
        }
    }

    fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('n') => {
                self.state = State::CreatePlayList;
            }
            _ => (),
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .direction(ratatui::layout::Direction::Vertical)
            .split(area);

        unimplemented!()
    }
}

struct NewPlayList<'a> {
    textarea: TextArea<'a>,
    playlistname: PlaylistName,
    backend: Arc<Backend>,
}

impl<'a> NewPlayList<'a> {
    fn new(backend: Arc<Backend>) -> Self {
        Self {
            textarea: TextArea::default(),
            playlistname: String::new(),
            backend,
        }
    }

    fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let lines = self.textarea.lines()[0].trim();
                if !lines.is_empty() {
                    self.playlistname = lines.to_owned();
                    self.backend
                        .PlayListManager
                        .create_playlist(&self.playlistname)
                        .is_ok();
                }
            }
            _ => {
                self.textarea.input(key);
            }
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let search_block = Block::default()
            .title("Create New PlayList")
            .borders(Borders::ALL);
        self.textarea.set_cursor_line_style(Style::default());
        self.textarea.set_placeholder_text("Enter PlayListName");
        self.textarea.set_style(Style::default().fg(Color::White));
        self.textarea.set_block(search_block);
        self.textarea.render(area, buf);
    }
}
