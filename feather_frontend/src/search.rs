use crate::backend::{Backend, Song};
use crossterm::event::{KeyCode, KeyEvent};
use feather::{ArtistName, SongId, SongName};
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
use std::sync::Arc;
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
    tx_player: mpsc::Sender<bool>, // Channel to communicate with player
    backend: Arc<Backend>,         // Audio backend for search and playback
    vertical_scroll_state: ScrollbarState, // Vertical scrollbar state
    display_content: bool,         // Flag to show search results
    results: Result<Option<Vec<((SongName, SongId), Vec<ArtistName>)>>, String>, // Search results or error
    selected: usize,             // Index of selected result
    selected_song: Option<Song>, // Currently selected song details
    max_len: Option<usize>,      // Total number of search results
}

impl Search<'_> {
    // Constructor initializing the Search struct
    pub fn new(backend: Arc<Backend>, tx_player: mpsc::Sender<bool>) -> Self {
        let (tx, rx) = mpsc::channel(32); // Create channel for async search results
        Self {
            query: String::new(),
            state: SearchState::SearchBar,
            textarea: TextArea::default(),
            tx,
            rx,
            tx_player,
            backend,
            vertical_scroll_state: ScrollbarState::default(),
            display_content: false,
            results: Ok(None),
            selected: 0,
            selected_song: None,
            max_len: None,
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
            // SearchResults state
            match key.code {
                KeyCode::Tab => {
                    self.change_state();
                } // Switch to search bar
                KeyCode::Char('j') | KeyCode::Down => {
                    // Move selection down
                    self.selected = self.selected.saturating_add(1);
                    if let Some(len) = self.max_len {
                        self.selected = self.selected.min(len - 1);
                    }
                    self.vertical_scroll_state = self.vertical_scroll_state.position(self.selected);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    // Move selection up
                    self.selected = self.selected.saturating_sub(1);
                    self.vertical_scroll_state = self.vertical_scroll_state.position(self.selected);
                }
                KeyCode::Enter => {
                    // Play selected song
                    if let Some(song) = self.selected_song.clone() {
                        let backend = self.backend.clone();
                        let tx_player = self.tx_player.clone();
                        tokio::spawn(async move {
                            let _ = backend.play_music(song).await.is_ok();
                            let _ = tx_player.send(true).await;
                        });
                    }
                }
                _ => {}
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
                                    Some(Song::new(song.clone(), songid.clone(), artists.clone()));
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
    }
}
