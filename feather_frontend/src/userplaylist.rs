#![allow(unused)]
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::PlaylistName;
use log::debug;
use log::log;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::prelude::StatefulWidget;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::Widget;
use std::collections::linked_list;
use std::sync::Arc;
use std::sync::Mutex;
use tui_textarea::TextArea;

use crate::backend::Backend;

enum State {
    AllPlayList,
    CreatePlayList,
    ViewPlayList,
}

pub struct UserPlayList<'a> {
    backend: Arc<Backend>,
    state: Arc<Mutex<State>>,
    new_playlist: NewPlayList<'a>,
    list_playlist: ListPlaylist,
    popup: Arc<Mutex<bool>>,
}

impl<'a> UserPlayList<'a> {
    pub fn new(backend: Arc<Backend>) -> Self {
        let popup = Arc::new(Mutex::new(false));
        let state = Arc::new(Mutex::new(State::AllPlayList));
        Self {
            backend: backend.clone(),
            list_playlist: ListPlaylist::new(backend.clone(), state.clone()),
            state,
            new_playlist: NewPlayList::new(backend, popup.clone()),
            popup: popup,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('`') => {
                if let Ok(mut state) = self.state.lock() {
                    *state = State::CreatePlayList;
                }
                if let Ok(mut popup) = self.popup.lock() {
                    debug!("{:?}", "popup_area");
                    *popup = true;
                }
            }
            _ => {
                if let Ok(state) = self.state.lock() {
                    match *state {
                        State::CreatePlayList => self.new_playlist.handle_keystrokes(key),
                        State::AllPlayList => self.list_playlist.handle_keystrokes(key),
                        _ => (),
                    }
                }
            }
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .direction(ratatui::layout::Direction::Horizontal)
            .split(area);
        let userplaylist_list_area = chunks[0];
        let viewplaylist_area = chunks[1];
        self.list_playlist.render(userplaylist_list_area, buf);

        if let Ok(value) = self.popup.try_lock() {
            if *value {
                drop(value);
                debug!("{:?}", "Should appear");
                let popup_area = Rect {
                    x: area.x + area.width / 3,  // 33% margin on both sides
                    y: area.y + area.height / 2, // Center it vertically
                    width: area.width / 3,       // 33% of the total width
                    height: 3,                   // Small height (1-3 lines)
                };

                self.new_playlist.render(popup_area, buf);
            }
        }
    }
}

struct NewPlayList<'a> {
    textarea: TextArea<'a>,
    playlistname: PlaylistName,
    popup: Arc<Mutex<bool>>,
    backend: Arc<Backend>,
}

impl<'a> NewPlayList<'a> {
    pub fn new(backend: Arc<Backend>, popup: Arc<Mutex<bool>>) -> Self {
        Self {
            textarea: TextArea::default(),
            playlistname: String::new(),
            backend,
            popup: popup,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let Ok(mut popup) = self.popup.lock() {
                    *popup = false;
                }
            }
            KeyCode::Enter => {
                let lines = self.textarea.lines()[0].trim();
                if !lines.is_empty() {
                    self.playlistname = lines.to_owned();
                    if self
                        .backend
                        .PlayListManager
                        .create_playlist(&self.playlistname)
                        .is_ok()
                    {
                        if let Ok(mut popup) = self.popup.lock() {
                            *popup = false;
                        }
                    }
                }
            }
            _ => {
                self.textarea.input(key);
            }
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // debug!("{:?}", "Should appear 2");
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

struct ListPlaylist {
    backend: Arc<Backend>,
    selected: usize,
    max_len: usize,
    vertical_scroll_state: ScrollbarState,
}

impl ListPlaylist {
    fn new(backend: Arc<Backend>, state: Arc<Mutex<State>>) -> Self {
        ListPlaylist {
            backend,
            selected: 0,
            max_len: 0,
            vertical_scroll_state: ScrollbarState::default(),
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                // Move selection down
                self.select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Move selection up
                self.select_previous();
            }
            _ => (),
        }
    }

    // Moves selection to next item, respecting bounds
    fn select_next(&mut self) {
        if self.max_len > 0 {
            self.selected = (self.selected + 1).min(self.max_len - 1);
            self.vertical_scroll_state = self.vertical_scroll_state.position(self.selected);
        }
    }

    // Moves selection to previous item, preventing underflow
    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.selected);
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        scrollbar.render(area, buf, &mut self.vertical_scroll_state);

        if let Ok(playlist_names) = self.backend.PlayListManager.list_playlists() {
            self.max_len = playlist_names.len();
            let view_items: Vec<ListItem> = playlist_names
                .into_iter()
                .enumerate()
                .map(|(i, item)| {
                    // Format each item for display
                    let is_selected = i == self.selected;
                    let style = if is_selected {
                        // Highlight selected item
                        Style::default().fg(Color::Yellow).bg(Color::Blue)
                    } else {
                        Style::default()
                    };
                    let text = format!("{}", item);
                    ListItem::new(Span::styled(text, style))
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(self.selected));
            StatefulWidget::render(
                // Render the list
                List::new(view_items)
                    .block(Block::default().borders(Borders::ALL))
                    .highlight_symbol("▶"),
                area,
                buf,
                &mut list_state,
            );
        }
        let outer_block = Block::default().borders(Borders::ALL);
        outer_block.render(area, buf);
    }
}
