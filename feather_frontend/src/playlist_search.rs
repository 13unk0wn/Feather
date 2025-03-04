#![allow(unused)]
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::prelude::Buffer;
use ratatui::prelude::Rect;
use ratatui::prelude::StatefulWidget;
use ratatui::prelude::Widget;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarState;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::sleep;
use tui_textarea::TextArea;

use crate::backend::Backend;
enum PlayListSearchState {
    Search,
    ViewSelectedPlaylist,
}

pub struct PlayListSearch<'a> {
    search: PlayListSearchComponent<'a>,
    state: PlayListSearchState,
}

impl<'a> PlayListSearch<'a> {
    pub fn new(backend: Arc<Backend>) -> Self {
        Self {
            search: PlayListSearchComponent::new(backend),
            state: PlayListSearchState::Search,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            _ => match self.state {
                PlayListSearchState::Search => {
                    self.search.handle_keystrokes(key);
                }
                _ => (),
            },
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .direction(ratatui::layout::Direction::Horizontal)
            .split(area);
        self.search.render(chunks[0], buf);
    }
}

#[derive(PartialEq)]
enum PlayListSearchComponentState {
    SearchBar,
    SearchResult,
}

struct PlayListSearchComponent<'a> {
    textarea: TextArea<'a>,
    query: String,
    state: PlayListSearchComponentState,
    display_content: bool,
    selected: usize,
    backend: Arc<Backend>,
    tx: mpsc::Sender<Result<Vec<((String, String), Vec<String>)>, String>>,
    rx: mpsc::Receiver<Result<Vec<((String, String), Vec<String>)>, String>>,
    results: Result<Option<Vec<((String, String), Vec<String>)>>, String>,
    verticle_scrollbar: ScrollbarState,
    max_len: Option<usize>,
}

impl<'a> PlayListSearchComponent<'a> {
    fn new(backend: Arc<Backend>) -> Self {
        let (tx, rx) = mpsc::channel(32);
        Self {
            textarea: TextArea::default(),
            query: String::new(),
            state: PlayListSearchComponentState::SearchBar,
            display_content: false,
            selected: 0,
            tx,
            rx,
            backend,
            results: Ok(None),
            verticle_scrollbar: ScrollbarState::default(),
            max_len: None,
        }
    }
    fn change_state(&mut self) {
        if self.state == PlayListSearchComponentState::SearchBar {
            self.state = PlayListSearchComponentState::SearchResult;
        } else {
            self.state = PlayListSearchComponentState::SearchBar;
        }
    }

    fn handle_keystrokes_search_bar(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                // println!("Enter pressed");
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
                        match backend.yt.fetch_playlist(&query).await {
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
            }
        }
    }
    fn handle_keystrokes_search_result(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                // Move selection down
                self.selected = self.selected.saturating_add(1);
                if let Some(len) = self.max_len {
                    self.selected = self.selected.min(len - 1);
                }
                self.verticle_scrollbar = self.verticle_scrollbar.position(self.selected);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Move selection up
                self.selected = self.selected.saturating_sub(1);
                self.verticle_scrollbar = self.verticle_scrollbar.position(self.selected);
            }
            _ => (),
        }
    }
    fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => self.change_state(),
            _ => match self.state {
                PlayListSearchComponentState::SearchBar => self.handle_keystrokes_search_bar(key),
                PlayListSearchComponentState::SearchResult => {
                    self.handle_keystrokes_search_result(key)
                }
            },
        }
    }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search bar height
                Constraint::Min(0),    // Results area
            ])
            .split(area);

        if let Ok(response) = self.rx.try_recv() {
            if let Ok(result) = response {
                self.results = Ok(Some(result));
            } else if let Err(e) = response {
                self.results = Err(e);
            }
            self.display_content = true;
        }

        let searchbar_area = chunks[0];
        let results_area = chunks[1];
        let search_block = Block::default().title("Search Music").borders(Borders::ALL);
        self.textarea.set_cursor_line_style(Style::default());
        self.textarea.set_placeholder_text("Search Playlist");
        self.textarea.set_style(Style::default().fg(Color::White));
        self.textarea.set_block(search_block);
        self.textarea.render(searchbar_area, buf);

        // Render vertical scrollbar
        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
        vertical_scrollbar.render(results_area, buf, &mut self.verticle_scrollbar);

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
                                // self.selected_song =
                                // Some(Song::new(song.clone(), songid.clone(), artists.clone()));
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
        let outer_block = Block::default().borders(Borders::ALL);
        outer_block.render(area, buf);
    }
}
