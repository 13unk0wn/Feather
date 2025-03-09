#![allow(unused)]
use ratatui::prelude::Alignment;
use ratatui::prelude::Direction;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::database::SongDatabase;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::prelude::Rect;

use ratatui::prelude::Buffer;

use crate::playlist_search;
use crate::playlist_search::PlayListSearch;
use crate::search::Search;

#[derive(PartialEq, Debug)]
enum SearchMainState {
    SongSearch,
    PlayListSearch,
}

pub struct SearchMain<'a> {
    state: SearchMainState,
    search: Search<'a>,
    playlist_search: PlayListSearch<'a>,
}

impl<'a> SearchMain<'a> {
    pub fn new(search: Search<'a>, playlist_search: PlayListSearch<'a>) -> Self {
        SearchMain {
            state: SearchMainState::SongSearch,
            search,
            playlist_search,
        }
    }
    fn change_state(&mut self) {
        if self.state == SearchMainState::SongSearch {
            self.state = SearchMainState::PlayListSearch;
        } else {
            self.state = SearchMainState::SongSearch;
        }
    }
    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(';') => self.change_state(),
            _ => match self.state {
                SearchMainState::SongSearch => self.search.handle_keystrokes(key),
                _ => self.playlist_search.handle_keystrokes(key),
            },
        }
    }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let s = format!("Current Search Mode : {:?}", self.state);
        let block = Paragraph::new(s)
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL));

        block.render(chunks[0], buf);

        match self.state {
            SearchMainState::SongSearch => self.search.render(chunks[1], buf),
            _ => self.playlist_search.render(chunks[1], buf),
        }
    }
}
