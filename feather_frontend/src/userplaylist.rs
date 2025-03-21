#![allow(unused)]
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::PlaylistName;
use feather::database::PAGE_SIZE;
use feather::database::Song;
use feather::database::SongDatabase;
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
use simplelog::Config;
use std::collections::linked_list;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::backend::Backend;
use crate::config;
use crate::config::USERCONFIG;

#[derive(PartialEq)]
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
    pub fn new(
        backend: Arc<Backend>,
        tx_play: mpsc::Sender<Arc<Mutex<SongDatabase>>>,
        config: Rc<USERCONFIG>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(1);
        let (tx_playlist, rx_playlist) = mpsc::channel(32);
        let popup = Arc::new(Mutex::new(false));
        let state = State::AllPlayList;
        Self {
            backend: backend.clone(),
            list_playlist: ListPlaylist::new(backend.clone(), tx_playlist, config.clone()),
            viewplaylist: ViewPlayList::new(rx_playlist, backend.clone(), tx_play, config.clone()),
            state,
            new_playlist: NewPlayList::new(backend, popup.clone(), tx, config),
            popup: popup,
            rx,
        }
    }

    fn change_state(&mut self) {
        if self.state == State::ViewPlayList {
            self.state = State::AllPlayList;
        } else if self.state == State::AllPlayList {
            self.state = State::ViewPlayList;
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
            KeyCode::Char('[') => {
                self.change_state();
            }
            _ => match self.state {
                State::CreatePlayList => self.new_playlist.handle_keystrokes(key),
                State::ViewPlayList => self.viewplaylist.handle_keystrokes(key),
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
        self.viewplaylist.render(viewplaylist_area, buf);

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
    config: Rc<USERCONFIG>,
}

impl<'a> NewPlayList<'a> {
    pub fn new(
        backend: Arc<Backend>,
        popup: Arc<Mutex<bool>>,
        tx: mpsc::Sender<bool>,
        config: Rc<USERCONFIG>,
    ) -> Self {
        Self {
            textarea: TextArea::default(),
            playlistname: String::new(),
            backend,
            popup: popup,
            tx,
            config,
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
        let bg_color = self.config.bg_color;
        let text_color = self.config.text_color;
        let global_style = Style::default()
            .fg(Color::Rgb(text_color.0, text_color.1, text_color.2))
            .bg(Color::Rgb(bg_color.0, bg_color.1, bg_color.2));
        Block::default().style(global_style).render(area, buf);
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
    config: Rc<USERCONFIG>,
}

impl ListPlaylist {
    fn new(backend: Arc<Backend>, tx: mpsc::Sender<String>, config: Rc<USERCONFIG>) -> Self {
        ListPlaylist {
            backend,
            selected: 0,
            max_len: 0,
            vertical_scroll_state: ScrollbarState::default(),
            selected_playlist_name: None,
            tx,
            config,
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
                        // Highlight selected item
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

struct ViewPlayList {
    rx: mpsc::Receiver<String>,
    content: Arc<Mutex<Option<Vec<Song>>>>,
    db: Arc<Mutex<Option<SongDatabase>>>,
    backend: Arc<Backend>,
    playlist_name: Option<String>,
    verticle_scrollbar: ScrollbarState,
    selected: usize,
    max_len: usize,
    offset: usize,
    max_page: Arc<Mutex<Option<usize>>>,
    tx_playlist: mpsc::Sender<Arc<Mutex<SongDatabase>>>,
    config: Rc<USERCONFIG>,
}

impl ViewPlayList {
    fn new(
        rx: mpsc::Receiver<String>,
        backend: Arc<Backend>,
        tx_playlist: mpsc::Sender<Arc<Mutex<SongDatabase>>>,
        config: Rc<USERCONFIG>,
    ) -> Self {
        Self {
            rx,
            content: Arc::new(Mutex::new(None)),
            db: Arc::new(Mutex::new(None)),
            backend,
            verticle_scrollbar: ScrollbarState::default(),
            selected: 0,
            max_len: PAGE_SIZE,
            playlist_name: None,
            offset: 0,
            max_page: Arc::new(Mutex::new(None)),
            tx_playlist,
            config,
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
                    }
                });
            }
            KeyCode::Enter => {
                let db = self.db.clone();
                let backend = self.backend.clone();
                let select = self.selected;
                tokio::spawn(async move {
                    // Extract the SongDatabase before awaiting
                    let db_inner = {
                        let db_guard = db.lock().expect("Failed to lock db");
                        db_guard.clone() // Clone the Option<SongDatabase>
                    };

                    if let Some(db_inner) = db_inner {
                        backend.play_playlist(db_inner, select).await;
                    }
                });
            }
            KeyCode::Right => {
                debug!("Calling next Page");
                if let Ok(db) = self.db.lock() {
                    if let Some(db) = db.clone() {
                        if let Ok(max_page) = self.max_page.lock() {
                            let total_pages = max_page.unwrap_or(0);
                            let new_offset = (self.offset + PAGE_SIZE).min(total_pages);

                            if new_offset != self.offset {
                                debug!("Calling next Page 2 ");
                                if let Ok(iter_db) = db.next_page(new_offset) {
                                    let new_vec: Vec<Song> = iter_db.into_iter().collect();
                                    if !new_vec.is_empty() {
                                        if let Ok(mut content) = self.content.lock() {
                                            *content = Some(new_vec);
                                            debug!("Changed  content");
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
        if let Ok(name) = self.rx.try_recv() {
            self.playlist_name = Some(name.clone());
            if let Ok(playlist) = self.backend.PlayListManager.convert_playlist(&name) {
                let page_size =  PAGE_SIZE;
                let len_clone = self.max_page.clone();
                if let Ok(mut l) = len_clone.lock() {
                    let value = ((playlist.db.len() + page_size - 1) / page_size) * page_size;
                    *l = Some(value);
                }
                if let Ok(mut db) = self.db.lock() {
                    *db = Some(playlist);
                }
            }
            if let Ok(playlist) = self.db.lock() {
                if let Some(p) = playlist.clone() {
                    drop(playlist);
                    self.offset = 0;
                    self.selected = 0;
                    if let Ok(songs) = p.next_page(self.offset) {
                        if let Ok(mut songs_list) = self.content.lock() {
                            if songs.len() > 0 {
                                *songs_list = Some(songs);
                            }
                        }
                    }
                }
            }
        }
        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
        let selected_item_text_color = self.config.selected_list_item;
        let selected_item_bg = self.config.selected_tab_color;
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
                        .highlight_symbol(&self.config.selected_item_char),
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
