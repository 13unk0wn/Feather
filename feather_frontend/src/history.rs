#![allow(unused)]
use crate::backend::Backend;
use crate::popup_playlist::PopUpAddPlaylist;
use crossterm::event::{KeyCode, KeyEvent};
use feather::config::USERCONFIG;
use feather::database::HISTORY_PAGE_SIZE;
use feather::database::HistoryDB;
use feather::database::Song;
use ratatui::prelude::{Buffer, Color, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarState,
    StatefulWidget, Widget,
};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

// Defines a struct to manage playback history UI
pub struct History {
    history: Arc<HistoryDB>,               // Database connection for history
    selected: usize,                       // Index of currently selected item
    vertical_scroll_state: ScrollbarState, // State for vertical scrollbar
    max_len: usize,                        // Total number of history items
    selected_song: Option<Song>,           // Currently selected song details
    backend: Arc<Backend>,                 // Audio backend for playback
    tx_song: mpsc::Sender<Song>,
    popup_appear: bool,
    popup: PopUpAddPlaylist,
    rx_signal: mpsc::Receiver<bool>,
    config: Rc<USERCONFIG>,
    offset: usize,
}

impl History {
    // Constructor initializing the History struct
    pub fn new(history: Arc<HistoryDB>, backend: Arc<Backend>, config: Rc<USERCONFIG>) -> Self {
        let (tx_song, rx_song) = mpsc::channel(8);
        let (tx_signal, rx_signal) = mpsc::channel(1);
        Self {
            history,
            selected: 0,
            vertical_scroll_state: ScrollbarState::default(),
            max_len: 0,
            selected_song: None,
            backend: backend.clone(),
            tx_song,
            popup_appear: false,
            popup: PopUpAddPlaylist::new(backend, rx_song, tx_signal, config.clone()),
            rx_signal,
            config,
            offset: 0,
        }
    }

    // Handles keyboard input for navigation and actions
    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        let mut value = true;
        if self.popup_appear {
            self.popup.handle_keystrokes(key);
            value = false;
        }
        if value {
            match key.code {
                KeyCode::Right => {
                    if self.backend.history.db.len() >= self.offset + HISTORY_PAGE_SIZE {
                        self.offset += HISTORY_PAGE_SIZE;
                        self.selected = 0;
                    }
                }
                KeyCode::Left => {
                    self.selected = 0;
                    self.offset = self.offset.saturating_sub(HISTORY_PAGE_SIZE);
                }
                KeyCode::Char('a') => {
                    if let Some(song) = self.selected_song.clone() {
                        let tx = self.tx_song.clone();
                        tokio::spawn(async move {
                            tx.send(song).await;
                        });
                        self.popup_appear = true;
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
                KeyCode::Char('d') => {
                    // Delete selected entry
                    if let Some(song) = &self.selected_song {
                        let _ = self.history.delete_entry(&song.id);
                    }
                }
                KeyCode::Enter => {
                    // Play selected song
                    if let Some(song) = self.selected_song.clone() {
                        let backend = Arc::clone(&self.backend);
                        tokio::spawn(async move {
                            // Spawn async task for playback
                            if backend.play_music(song, false).await.is_ok() {}
                        });
                    }
                }
                _ => (), // Ignore other keys
            }
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

    // Renders the history UI component
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if let Ok(_) = self.rx_signal.try_recv() {
            self.popup_appear = false;
        }
        // Setup history list area with scrollbar
        let history_area = area;
        let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        scrollbar.render(history_area, buf, &mut self.vertical_scroll_state);

        let selected_item_text_color = self.config.selected_list_item;
        let selected_item_bg = self.config.selected_tab_color;
        // Fetch and render history items
        if let Ok(items) = self.history.get_history(self.offset) {
            if items.len() == 0 {
                self.offset = self.offset.saturating_sub(HISTORY_PAGE_SIZE);
            }
            self.max_len = items.len();
            self.vertical_scroll_state = self.vertical_scroll_state.content_length(self.max_len);

            let view_items: Vec<ListItem> = items
                .into_iter()
                .enumerate()
                .map(|(i, item)| {
                    // Format each item for display
                    let is_selected = i == self.selected;
                    if is_selected {
                        self.selected_song = Some(Song::new(
                            item.song_id.clone(),
                            item.song_name.clone(),
                            item.artist_name.clone(),
                        ));
                    }
                    let style = if is_selected {
                        // Highlight selected item
                        Style::default()
                            .fg(Color::Rgb(
                                selected_item_text_color.0,
                                selected_item_text_color.1,
                                selected_item_text_color.0,
                            ))
                            .bg(Color::Rgb(
                                selected_item_bg.0,
                                selected_item_bg.1,
                                selected_item_bg.2,
                            ))
                    } else {
                        Style::default()
                    };
                    let text = format!("{} - {}", item.song_name, item.artist_name.join(", "));
                    ListItem::new(Span::styled(text, style))
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(self.selected));
            StatefulWidget::render(
                // Render the list
                List::new(view_items)
                    .block(Block::default().borders(Borders::ALL))
                    .highlight_symbol(&self.config.selected_item_char),
                history_area,
                buf,
                &mut list_state,
            );
        } else {
            // Handle history loading failure
            self.max_len = 0;
            self.selected = 0;
            Paragraph::new("Failed to load history").render(history_area, buf);
        }
        if self.popup_appear {
            let popup_area = Rect {
                x: area.x + area.width / 4, // 25% margin on both sides (centers the popup)
                y: area.y + area.height / 4, // 25% margin on top and bottom (centers it)
                width: area.width / 2,      // 50% of the total width
                height: area.height / 2,    // 50% of the total height
            };

            self.popup.render(popup_area, buf);
        }
    }
}
