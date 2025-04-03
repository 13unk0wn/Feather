#![allow(unused)]
use feather::config::KeyConfig;
use feather::config::USERCONFIG;
use ratatui::prelude::Alignment;
use ratatui::prelude::Direction;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use simplelog::Config;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::database::SongDatabase;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::prelude::Rect;

use ratatui::prelude::Buffer;

use crate::playlist_search;
use crate::playlist_search::PlayListSearch;
use crate::search::Search;

#[derive(PartialEq, Debug, Copy, Clone)]
enum SearchMainState {
    SongSearch,
    PlayListSearch,
}

pub struct SearchMain<'a> {
    state: SearchMainState,
    search: Search<'a>,
    playlist_search: PlayListSearch<'a>,
    key_config: Rc<KeyConfig>,
    config: Rc<USERCONFIG>,
}

impl<'a> SearchMain<'a> {
    pub fn new(
        search: Search<'a>,
        playlist_search: PlayListSearch<'a>,
        key_config: Rc<KeyConfig>,
        config: Rc<USERCONFIG>,
    ) -> Self {
        SearchMain {
            state: SearchMainState::SongSearch,
            search,
            playlist_search,
            key_config,
            config,
        }
    }
    fn change_state(&mut self) {
        if self.state == SearchMainState::SongSearch {
            self.state = SearchMainState::PlayListSearch;
        } else {
            self.state = SearchMainState::SongSearch;
        }
    }

    pub fn show_keystokes(&mut self, area: Rect, buf: &mut Buffer) {
        let state = self.state;
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20), // Space for the keystroke bar
                Constraint::Percentage(60), // Empty space at the top
                Constraint::Percentage(10), // Space for the keystroke bar
            ])
            .split(area);
        let status_block = Block::default().borders(Borders::TOP);

        let switch = self.key_config.search.switch;
        let up = self
            .key_config
            .search
            .up
            .unwrap_or(self.key_config.default.up);
        let down = self
            .key_config
            .search
            .down
            .unwrap_or(self.key_config.default.down);
        let color = self.config.selected_tab_color;
        match self.state {
            SearchMainState::SongSearch => {
                let search_switch = self.key_config.search.song.switch_mode.unwrap_or('t');
                let mut search_switch_str = search_switch.to_string();
                let add_to_playlist = self.key_config.default.add_to_playlist;
                let play_song = self.key_config.default.play_song;

                if search_switch == 't' {
                    search_switch_str = "TAB".to_string();
                }

                let keystroke_bar = Line::from(vec![
                    Span::styled(
                        format!("[({}/▲)/({}/▼)→Navigation] ", up, down),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}→toggle song_search_mode] ", search_switch_str),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}→add_to_playlist] ", add_to_playlist),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}→add_to_playlist] ", add_to_playlist),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}/ENTER→play_song] ", play_song),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                ]);
                status_block
                    .title(keystroke_bar)
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .render(vertical_layout[1], buf);
            }
            SearchMainState::PlayListSearch => match self.playlist_search.state {
                playlist_search::PlayListSearchState::Search => {
                    let switch = self.key_config.search.playlist.switch_mode;
                    let search_switch = self
                        .key_config
                        .search
                        .playlist
                        .playlist_search
                        .switch_mode
                        .unwrap_or('t');
                    let mut search_switch_str = search_switch.to_string();
                    if search_switch == 't' {
                        search_switch_str = "TAB".to_string();
                    }
                    let add_to_playlist = self.key_config.default.add_to_playlist;
                    let select_playlist = self
                        .key_config
                        .search
                        .playlist
                        .playlist_search
                        .select_playlist
                        .unwrap_or(self.key_config.default.play_song);

                    let keystroke_bar = Line::from(vec![
                        Span::styled(
                            format!("[{}→View playlist] ", switch),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                        Span::styled(
                            format!("[({}/▲)/({}/▼)→Navigation] ", up, down),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                        Span::styled(
                            format!("[{}→toggle playlist_search_mode] ", search_switch_str),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                        Span::styled(
                            format!("[{}/ENTER→play_song] ", select_playlist),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                    ]);
                    status_block
                        .title(keystroke_bar)
                        .title_alignment(ratatui::layout::Alignment::Center)
                        .render(vertical_layout[1], buf);
                }
                playlist_search::PlayListSearchState::ViewSelectedPlaylist => {
                    let switch = self.key_config.search.playlist.switch_mode;

                    let start_playlist =
                        self.key_config.search.playlist.view_playlist.start_playlist;
                    let start_from_here = self
                        .key_config
                        .search
                        .playlist
                        .view_playlist
                        .start_from_here;

                    let next_page = self
                        .key_config
                        .search
                        .playlist
                        .view_playlist
                        .next_page
                        .unwrap_or(self.key_config.default.next_page);
                    let prev_page = self
                        .key_config
                        .search
                        .playlist
                        .view_playlist
                        .prev_page
                        .unwrap_or(self.key_config.default.prev_page);

                    let keystroke_bar = Line::from(vec![
                        Span::styled(
                            format!("[{}→Search playlist] ", switch),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                        Span::styled(
                            format!("[{}→Start playlist] ", start_playlist),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                        Span::styled(
                            format!("[{}/ENTER→start_from_here] ", start_from_here),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                        Span::styled(
                            format!("[({}/→)→next_page] ", next_page),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                        Span::styled(
                            format!("[({}/←)→prev_page]", prev_page),
                            Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                        ),
                    ]);
                    status_block
                        .title(keystroke_bar)
                        .title_alignment(ratatui::layout::Alignment::Center)
                        .render(vertical_layout[1], buf);
                }
            },
        }
    }
    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) if key.code == KeyCode::Char(self.key_config.search.switch) => {
                self.change_state();
            }
            _ => match self.state {
                SearchMainState::SongSearch => {
                    self.search.handle_keystrokes(key, self.key_config.clone())
                }
                _ => self
                    .playlist_search
                    .handle_keystrokes(key, self.key_config.clone()),
            },
        }
    }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .constraints([Constraint::Min(0)])
            .split(area);

        match self.state {
            SearchMainState::SongSearch => self.search.render(chunks[0], buf),
            _ => self.playlist_search.render(chunks[0], buf),
        }
    }
}
