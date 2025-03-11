#![allow(unused)]
use crate::backend::Backend;
use crossterm::event::{KeyCode, KeyEvent};
use feather::{ArtistName, SongId, SongName};
use feather::{PlaylistName, database::Song};
use ratatui::widgets::Clear;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarState,
        StatefulWidget, Widget,
    },
};
use std::sync::{Arc, Mutex};
use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};
use tui_textarea::TextArea;

// Defines possible states for the search interface
enum SearchState {
    SearchBar,     // When focused on input field
    SearchResults, // When browsing search results
}

pub struct Search<'a> {
    textarea: TextArea<'a>, // Text input widget for search queries
    state: SearchState,     // Current UI state
    query: String,          // Current search query text
    tx: mpsc::Sender<Result<Vec<((String, String), Vec<String>)>, String>>, // Sender for search results
    rx: mpsc::Receiver<Result<Vec<((String, String), Vec<String>)>, String>>, // Receiver for search results
    backend: Arc<Backend>, // Audio backend for search and playback
    vertical_scroll_state: ScrollbarState, // Vertical scrollbar state
    display_content: bool, // Flag to show search results
    results: Result<Option<Vec<((SongName, SongId), Vec<ArtistName>)>>, String>, // Search results or error
    selected: usize,             // Index of selected result
    selected_song: Option<Song>, // Currently selected song details
    max_len: Option<usize>,      // Total number of search results
    popup_appear: Arc<Mutex<bool>>,
    popup: PopUpAddPlaylist,
    tx_song: mpsc::Sender<Song>,
}

impl Search<'_> {
    // Constructor initializing the Search struct
    pub fn new(backend: Arc<Backend>) -> Self {
        let (tx, rx) = mpsc::channel(32); // Create channel for async search results
        let (tx_song, rx_song) = mpsc::channel(8);
        let popup_appear = Arc::new(Mutex::new(false));
        Self {
            query: String::new(),
            state: SearchState::SearchBar,
            textarea: TextArea::default(),
            tx,
            rx,
            backend: backend.clone(),
            vertical_scroll_state: ScrollbarState::default(),
            display_content: false,
            results: Ok(None),
            selected: 0,
            selected_song: None,
            max_len: None,
            tx_song,
            popup: PopUpAddPlaylist::new(backend, rx_song),
            popup_appear,
        }
    }

    // Handles keyboard input based on current state
    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        if let SearchState::SearchBar = self.state {
            match key.code {
                KeyCode::Tab => {
                    // Switch to results state
                    self.change_state();
                }
                KeyCode::Enter => {
                    // Execute search
                    self.display_content = false;
                    self.selected = 0;
                    let text = self.textarea.lines();
                    if !text.is_empty() {
                        self.query = text[0].trim().to_string();
                        let tx = self.tx.clone();
                        let query = self.query.clone();
                        let backend = self.backend.clone();
                        tokio::spawn(async move {
                            // Async task for search
                            sleep(Duration::from_millis(500)).await; // Debounce
                            match backend.yt.search(&query).await {
                                Ok(songs) => {
                                    let _ = tx.send(Ok(songs)).await;
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(e)).await;
                                }
                            }
                        });
                    }
                }
                _ => {
                    self.textarea.input(key);
                } // Handle text input
            }
        } else {
            let mut value = true;
            if let Ok(popup) = self.popup_appear.lock() {
                if *popup {
                    self.popup.handle_keystrokes(key);
                    value = false;
                    drop(popup);
                }
            }
            if value {
                // SearchResults state
                match key.code {
                    KeyCode::Tab => {
                        self.change_state();
                    } // Switch to search bar
                    KeyCode::Char('a') => {
                        if let Some(song) = self.selected_song.clone() {
                            let tx = self.tx_song.clone();
                            tokio::spawn(async move {
                                tx.send(song).await;
                            });
                            if let Ok(mut popup) = self.popup_appear.lock(){
                                *popup = true;
                            }
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        // Move selection down
                        self.selected = self.selected.saturating_add(1);
                        if let Some(len) = self.max_len {
                            self.selected = self.selected.min(len - 1);
                        }
                        self.vertical_scroll_state =
                            self.vertical_scroll_state.position(self.selected);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        // Move selection up
                        self.selected = self.selected.saturating_sub(1);
                        self.vertical_scroll_state =
                            self.vertical_scroll_state.position(self.selected);
                    }
                    KeyCode::Enter => {
                        // Play selected song
                        if let Some(song) = self.selected_song.clone() {
                            let backend = self.backend.clone();
                            tokio::spawn(async move {
                                let _ = backend.play_music(song, false).await.is_ok();
                                // let _ = tx_player.send(true).await;
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Toggles between search bar and results view
    pub fn change_state(&mut self) {
        match self.state {
            SearchState::SearchResults => self.state = SearchState::SearchBar,
            _ => self.state = SearchState::SearchResults,
        }
    }

    // Renders the search UI
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar height
                Constraint::Min(0),    // Results area
                Constraint::Length(3), // Bottom bar
            ])
            .split(area);
        let searchbar_area = chunks[0];
        let results_area = chunks[1];
        let bottom_area = chunks[2];

        // Check for new search results
        if let Ok(response) = self.rx.try_recv() {
            if let Ok(result) = response {
                self.results = Ok(Some(result));
            } else if let Err(e) = response {
                self.results = Err(e);
            }
            self.display_content = true;
        }

        // Render search bar
        let search_block = Block::default().title("Search Music").borders(Borders::ALL);
        self.textarea.set_cursor_line_style(Style::default());
        self.textarea
            .set_placeholder_text("Search Song or Playlist");
        self.textarea.set_style(Style::default().fg(Color::White));
        self.textarea.set_block(search_block);
        self.textarea.render(searchbar_area, buf);

        // Render vertical scrollbar
        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
        vertical_scrollbar.render(results_area, buf, &mut self.vertical_scroll_state);

        // Render search results if available
        if self.display_content {
            if let Ok(result) = self.results.clone() {
                if let Some(r) = result {
                    self.max_len = Some(r.len());
                    let items: Vec<ListItem> = r
                        .into_iter()
                        .enumerate()
                        .map(|(i, ((song, songid), artists))| {
                            // Format results
                            let style = if i == self.selected {
                                self.selected_song =
                                    Some(Song::new(songid.clone(), song.clone(), artists.clone()));
                                Style::default().fg(Color::Yellow).bg(Color::Blue)
                            } else {
                                Style::default()
                            };
                            let text = format!("{} - {}", song, artists.join(", "));
                            ListItem::new(Span::styled(text, style))
                        })
                        .collect();

                    let mut list_state = ListState::default();
                    list_state.select(Some(self.selected));
                    StatefulWidget::render(
                        // Render results list
                        List::new(items)
                            .block(Block::default().title("Results").borders(Borders::ALL))
                            .highlight_symbol("▶"),
                        results_area,
                        buf,
                        &mut list_state,
                    );
                }
            }
        }

        // Render bottom help bar
        let bottom_bar = Paragraph::new("Press '?' for Help in Global Mode")
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL));
        bottom_bar.render(bottom_area, buf); // Note: custom_area undefined, likely should be bottom_area

        // Render outer border
        let outer_block = Block::default().borders(Borders::ALL);
        outer_block.render(area, buf);
        if let Ok(value) = self.popup_appear.try_lock() {
            let v = *value;
            drop(value);
            if v {
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
}

struct PopUpAddPlaylist {
    backend: Arc<Backend>,
    max_len: usize,
    selected: usize,
    selected_playlist_name: Option<PlaylistName>,
    vertical_scroll_state: ScrollbarState, // Vertical scrollbar state
    selected_song: Option<Song>,
    rx: mpsc::Receiver<Song>,
}

impl PopUpAddPlaylist {
    fn new(
        backend: Arc<Backend>,
        rx: mpsc::Receiver<Song>,
    ) -> Self {
        Self {
            backend,
            max_len: 0,
            selected: 0,
            selected_playlist_name: None,
            vertical_scroll_state: ScrollbarState::default(),
            selected_song: None,
            rx,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if let Some(song) = &self.selected_song {
                    if let Some(playlist_name) = &self.selected_playlist_name {
                        self.backend
                            .PlayListManager
                            .add_song_to_playlist(&playlist_name, song.clone())
                            .is_ok();
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
