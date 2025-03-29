#![allow(unused)]
use crate::backend::Backend;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::PlaylistName;
use feather::config::USERCONFIG;
use feather::database::Song;
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
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarState;
use std::rc::Rc;
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
    config: Rc<USERCONFIG>,
}

impl PopUpAddPlaylist {
    pub fn new(
        backend: Arc<Backend>,
        rx: mpsc::Receiver<Song>,
        tx_signal: mpsc::Sender<bool>,
        config: Rc<USERCONFIG>,
    ) -> Self {
        Self {
            backend,
            max_len: 0,
            selected: 0,
            selected_playlist_name: None,
            vertical_scroll_state: ScrollbarState::default(),
            selected_song: None,
            rx,
            tx_signal,
            config,
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

        let bg_color = self.config.bg_color;
        let text_color = self.config.text_color;
        let global_style = Style::default()
            .fg(Color::Rgb(text_color.0, text_color.1, text_color.2))
            .bg(Color::Rgb(bg_color.0, bg_color.1, bg_color.2));

        Block::default().style(global_style).render(area, buf);

        // Render vertical scrollbar
        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
        vertical_scrollbar.render(area, buf, &mut self.vertical_scroll_state);
        let selected_item_text_color = self.config.selected_list_item;
        let selected_item_bg = self.config.selected_tab_color;
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
                    .highlight_symbol(&self.config.selected_item_char),
                area,
                buf,
                &mut list_state,
            );
        }
        let outer_block = Block::default().borders(Borders::ALL);
        outer_block.render(area, buf);
    }
}
