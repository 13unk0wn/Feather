use crate::backend::{Backend, Song};
use crossterm::event::{KeyCode, KeyEvent};
use feather::database::HistoryDB;
use ratatui::prelude::{Buffer, Color, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarState,
    StatefulWidget, Widget,
};
use std::sync::Arc;
use tokio::sync::mpsc;

// Defines a struct to manage playback history UI
pub struct History {
    history: Arc<HistoryDB>,               // Database connection for history
    selected: usize,                       // Index of currently selected item
    vertical_scroll_state: ScrollbarState, // State for vertical scrollbar
    max_len: usize,                        // Total number of history items
    selected_song: Option<Song>,           // Currently selected song details
    backend: Arc<Backend>,                 // Audio backend for playback
    tx_player: mpsc::Sender<bool>,         // Channel to communicate with player
}

impl History {
    // Constructor initializing the History struct
    pub fn new(
        history: Arc<HistoryDB>,
        backend: Arc<Backend>,
        tx_player: mpsc::Sender<bool>,
    ) -> Self {
        Self {
            history,
            selected: 0,
            vertical_scroll_state: ScrollbarState::default(),
            max_len: 0,
            selected_song: None,
            backend,
            tx_player,
        }
    }

    // Handles keyboard input for navigation and actions
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
            KeyCode::Char('d') => {
                // Delete selected entry
                if let Some(song) = &self.selected_song {
                    let _ = self.history.delete_entry(&song.song_id);
                }
            }
            KeyCode::Enter => {
                // Play selected song
                if let Some(song) = self.selected_song.clone() {
                    let backend = Arc::clone(&self.backend);
                    let tx_player = self.tx_player.clone();
                    tokio::spawn(async move {
                        // Spawn async task for playback
                        if backend.play_music(song).await.is_ok() {
                            let _ = tx_player.send(true).await;
                        }
                    });
                }
            }
            _ => (), // Ignore other keys
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
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)]) // Split layout
            .split(area);

        // Render title bar
        Paragraph::new("History")
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL))
            .render(chunks[0], buf);

        // Setup history list area with scrollbar
        let history_area = chunks[1];
        let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        scrollbar.render(history_area, buf, &mut self.vertical_scroll_state);

        // Fetch and render history items
        if let Ok(items) = self.history.get_history() {
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
                            item.song_name.clone(),
                            item.song_id.clone(),
                            item.artist_name.clone(),
                        ));
                    }
                    let style = if is_selected {
                        // Highlight selected item
                        Style::default().fg(Color::Yellow).bg(Color::Blue)
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
                    .highlight_symbol("▶"),
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
    }
}
