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
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::backend::Backend;

enum State {
    AllPlayList,
    CreatePlayList,
    ViewPlayList,
}

pub struct UserPlayList<'a> {
    backend: Arc<Backend>,
    state: State,
    new_playlist: NewPlayList<'a>,
    list_playlist: ListPlaylist,
    popup: Arc<Mutex<bool>>,
    viewplaylist: ViewPlayList,
    rx: mpsc::Receiver<bool>,
}

impl<'a> UserPlayList<'a> {
    pub fn new(backend: Arc<Backend>) -> Self {
        let (tx, rx) = mpsc::channel(1);
        let (tx_playlist, rx_playlist) = mpsc::channel(32);
        let popup = Arc::new(Mutex::new(false));
        let state = State::AllPlayList;
        Self {
            backend: backend.clone(),
            list_playlist: ListPlaylist::new(backend.clone(), tx_playlist),
            viewplaylist: ViewPlayList::new(backend.clone(), rx_playlist),
            state,
            new_playlist: NewPlayList::new(backend, popup.clone(), tx),
            popup: popup,
            rx,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('`') => {
                self.state = State::CreatePlayList;
                if let Ok(mut popup) = self.popup.lock() {
                    *popup = true;
                }
            }
            _ => match self.state {
                State::CreatePlayList => self.new_playlist.handle_keystrokes(key),
                State::AllPlayList => self.list_playlist.handle_keystrokes(key),
                _ => (),
            },
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

        if let Ok(value) = self.rx.try_recv() {
            self.state = State::AllPlayList;
        }

        if let Ok(value) = self.popup.try_lock() {
            if *value {
                drop(value);
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
    tx: mpsc::Sender<bool>,
}

impl<'a> NewPlayList<'a> {
    pub fn new(backend: Arc<Backend>, popup: Arc<Mutex<bool>>, tx: mpsc::Sender<bool>) -> Self {
        Self {
            textarea: TextArea::default(),
            playlistname: String::new(),
            backend,
            popup: popup,
            tx,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let Ok(mut popup) = self.popup.lock() {
                    *popup = false;
                }
                let tx = self.tx.clone();
                tokio::spawn(async move {
                    tx.send(true).await;
                });
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
                        let tx = self.tx.clone();
                        self.textarea.select_all();
                        self.textarea.cut();
                        tokio::spawn(async move {
                            tx.send(true).await;
                        });
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
    selected_playlist_name: Option<String>,
    tx: mpsc::Sender<String>,
}

impl ListPlaylist {
    fn new(backend: Arc<Backend>, tx: mpsc::Sender<String>) -> Self {
        ListPlaylist {
            backend,
            selected: 0,
            max_len: 0,
            vertical_scroll_state: ScrollbarState::default(),
            selected_playlist_name: None,
            tx,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Some(playlist_name) = self.selected_playlist_name.clone() {
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        tx.send(playlist_name).await;
                    });
                }
            }
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
                        self.selected_playlist_name = Some(item.clone());
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

struct ViewPlayList {
    backend: Arc<Backend>,
    playlist_name: Option<String>,
    rx: mpsc::Receiver<String>,
    offset: usize,
}

impl ViewPlayList {
    fn new(backend: Arc<Backend>, rx: mpsc::Receiver<String>) -> Self {
        Self {
            backend,
            rx,
            offset: 0,
            playlist_name: None,
        }
    }

    fn handle_keystrokes(&self) {}

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if let Ok(name) = self.rx.try_recv() {
            self.playlist_name = Some(name);
        }
        let outer_block = Block::default().borders(Borders::ALL);
        outer_block.render(area, buf);
    }
}
