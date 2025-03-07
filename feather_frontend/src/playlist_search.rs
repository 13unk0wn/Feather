#![allow(unused)]
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::database::PAGE_SIZE;
use feather::database::Song;
use feather::database::SongDatabase;
use ratatui::backend;
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
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio::time::sleep;
use tui_textarea::TextArea;

use crate::backend::Backend;
#[derive(PartialEq, PartialOrd)]
enum PlayListSearchState {
    Search,
    ViewSelectedPlaylist,
}

pub struct PlayListSearch<'a> {
    search: PlayListSearchComponent<'a>,
    view: SeletectPlayListView,
    state: PlayListSearchState,
}

impl<'a> PlayListSearch<'a> {
    pub fn new(backend: Arc<Backend>, tx_playlist: mpsc::Sender<Arc<Mutex<SongDatabase>>>) -> Self {
        let (tx_id, rx_id) = mpsc::channel(32);
        Self {
            search: PlayListSearchComponent::new(backend.clone(), tx_id),
            view: SeletectPlayListView::new(rx_id, backend, tx_playlist),
            state: PlayListSearchState::Search,
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('[') => self.change_state(),
            _ => match self.state {
                PlayListSearchState::Search => {
                    self.search.handle_keystrokes(key);
                }
                PlayListSearchState::ViewSelectedPlaylist => {
                    self.view.handle_keystrokes(key);
                }
                _ => (),
            },
        }
    }
    fn change_state(&mut self) {
        if self.state == PlayListSearchState::ViewSelectedPlaylist {
            self.state = PlayListSearchState::Search;
        } else {
            self.state = PlayListSearchState::ViewSelectedPlaylist;
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .direction(ratatui::layout::Direction::Horizontal)
            .split(area);
        self.search.render(chunks[0], buf);
        self.view.render(chunks[1], buf);
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
    selected_id: Option<String>,
    tx_id: mpsc::Sender<String>,
}

impl<'a> PlayListSearchComponent<'a> {
    fn new(backend: Arc<Backend>, tx_id: mpsc::Sender<String>) -> Self {
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
            selected_id: None,
            tx_id,
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
            KeyCode::Enter => {
                let tx_id = self.tx_id.clone();
                let id = self.selected_id.clone();
                tokio::spawn(async move {
                    if let Some(id) = id {
                        tx_id.send(id).await;
                    }
                });
            }
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
                                self.selected_id = Some(songid);
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

struct SeletectPlayListView {
    rx_id: mpsc::Receiver<String>,
    content: Arc<Mutex<Option<Vec<Song>>>>,
    db: Arc<Mutex<Option<SongDatabase>>>,
    backend: Arc<Backend>,
    verticle_scrollbar: ScrollbarState,
    selected: usize,
    max_len: usize,
    offset: usize,
    max_page: Arc<Mutex<Option<usize>>>,
    tx_playlist: mpsc::Sender<Arc<Mutex<SongDatabase>>>,
}

impl SeletectPlayListView {
    fn new(
        rx_id: mpsc::Receiver<String>,
        backend: Arc<Backend>,
        tx_playlist: mpsc::Sender<Arc<Mutex<SongDatabase>>>,
    ) -> Self {
        Self {
            rx_id,
            content: Arc::new(Mutex::new(None)),
            db: Arc::new(Mutex::new(None)),
            backend,
            verticle_scrollbar: ScrollbarState::default(),
            selected: 0,
            max_len: PAGE_SIZE,
            offset: 0,
            max_page: Arc::new(Mutex::new(None)),
            tx_playlist,
        }
    }

    fn handle_keystrokes(&mut self, key: KeyEvent) {
    match key.code {
        KeyCode::Char('p') => {
            let db = self.db.clone();
            let backend = self.backend.clone();
            tokio::spawn(async move {
                // Extract the SongDatabase before awaiting
                let db_inner = {
                    let db_guard = db.lock().expect("Failed to lock db");
                    db_guard.clone() // Clone the Option<SongDatabase>
                };
                
                if let Some(db_inner) = db_inner {
                    backend.play_playlist(db_inner, 0).await;
                    println!("-------------------Send------------------");
                }
            });
        }
        KeyCode::Right => {
            if let Ok(db) = self.db.lock() {
                if let Some(db) = db.clone() {
                    if let Ok(max_page) = self.max_page.lock() {
                        let total_pages = max_page.unwrap_or(0);
                        let new_offset = (self.offset + PAGE_SIZE).min(total_pages);

                        if new_offset != self.offset {
                            if let Ok(iter_db) = db.next_page(new_offset) {
                                let new_vec: Vec<Song> = iter_db.into_iter().collect();
                                if !new_vec.is_empty() {
                                    if let Ok(mut content) = self.content.lock() {
                                        *content = Some(new_vec);
                                        self.offset = new_offset;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Left => {
            if let Ok(db) = self.db.lock() {
                if let Some(db) = db.clone() {
                    let new_offset = self.offset.saturating_sub(PAGE_SIZE);

                    if new_offset != self.offset {
                        if let Ok(iter_db) = db.next_page(new_offset) {
                            let new_vec: Vec<Song> = iter_db.into_iter().collect();
                            if !new_vec.is_empty() {
                                if let Ok(mut content) = self.content.lock() {
                                    *content = Some(new_vec);
                                    self.offset = new_offset;
                                }
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            // Move selection down
            self.selected = self.selected.saturating_add(1);
            self.selected = self.selected.min(self.max_len - 1);
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

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if let Ok(id) = self.rx_id.try_recv() {
            self.offset = 0;
            self.selected = 0;
            let backend = self.backend.clone();
            let db = self.db.clone();
            let len_clone = self.max_page.clone();
            let content = self.content.clone();
            let page_size = PAGE_SIZE;
            tokio::spawn(async move {
                sleep(Duration::from_millis(500)).await; // Debounce
                match backend.yt.fetch_playlist_songs(id).await {
                    Ok(s) => {
                        if let Ok(mut db) = db.lock() {
                            let _ = db.take(); // drop the existing db
                        }
                        if let Ok(mut l) = len_clone.lock() {
                            let value = ((s.len() + page_size - 1) / page_size) * page_size;
                            *l = Some(value);
                        }
                        let mut db_temp = SongDatabase::new().expect("Failed to Form a Db");
                        for i in s {
                            let title = i.0.0;
                            let id = i.0.1;
                            let artist_name = i.1;
                            db_temp.add_song(title, id, artist_name);
                        }
                        let mut temp_vec = Vec::new();
                        if let Ok(db_iter) = db_temp.next_page(0) {
                            for song in db_iter {
                                temp_vec.push(song);
                            }
                        }
                        if let Ok(mut db) = db.lock() {
                            *db = Some(db_temp);
                        }
                        if let Ok(mut c) = content.lock() {
                            *c = Some(temp_vec);
                        }
                    }
                    _ => (),
                }
            });
        }
        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

        if let Ok(item) = self.content.lock() {
            if let Some(r) = item.clone() {
                self.max_len = r.len();
                if self.selected >= self.max_len {
                    self.selected = self.max_len - 1;
                }
                let items: Vec<ListItem> = r
                    .into_iter()
                    .enumerate()
                    .map(|(i, (song))| {
                        // Format results
                        let style = if i == self.selected {
                            Style::default().fg(Color::Yellow).bg(Color::Blue)
                        } else {
                            Style::default()
                        };
                        let text = format!("{} - {}", song.title, song.artist_name.join(", "));
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
                    area,
                    buf,
                    &mut list_state,
                );
            }
        }
        vertical_scrollbar.render(area, buf, &mut self.verticle_scrollbar);
        let outer_block = Block::default().borders(Borders::ALL);
        outer_block.render(area, buf);
    }
}
