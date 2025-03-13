#![allow(unused)]
use ratatui::widgets::List;
use crate::backend::Backend;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::database::Song;
use feather::PlaylistName;
use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::prelude::StatefulWidget;
use ratatui::prelude::Widget;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarState;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct PopUpAddPlaylist {
    backend: Arc<Backend>,
    max_len: usize,
    selected: usize,
    selected_playlist_name: Option<PlaylistName>,
    vertical_scroll_state: ScrollbarState, // Vertical scrollbar state
    selected_song: Option<Song>,
    rx: mpsc::Receiver<Song>,
    tx_signal: mpsc::Sender<bool>,
}

impl PopUpAddPlaylist {
    pub fn new(backend: Arc<Backend>, rx: mpsc::Receiver<Song>, tx_signal: mpsc::Sender<bool>) -> Self {
        Self {
            backend,
            max_len: 0,
            selected: 0,
            selected_playlist_name: None,
            vertical_scroll_state: ScrollbarState::default(),
            selected_song: None,
            rx,
            tx_signal,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                let tx_signal = self.tx_signal.clone();
                tokio::spawn(async move {
                    tx_signal.send(true).await;
                });
            }
            KeyCode::Enter => {
                if let Some(song) = &self.selected_song {
                    if let Some(playlist_name) = &self.selected_playlist_name {
                        self.backend
                            .PlayListManager
                            .add_song_to_playlist(&playlist_name, song.clone())
                            .is_ok();
                        let tx_signal = self.tx_signal.clone();
                        tokio::spawn(async move {
                            tx_signal.send(true).await;
                        });
                    }
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
        if let Ok(song) = self.rx.try_recv() {
            self.selected_song = Some(song);
        }
        Clear.render(area, buf);
        // Render vertical scrollbar
        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
        vertical_scrollbar.render(area, buf, &mut self.vertical_scroll_state);
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