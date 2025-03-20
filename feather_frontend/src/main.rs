#![allow(unused)]
use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, poll, read};
use feather::database::HistoryDB;
use feather_frontend::config::USERCONFIG;
use feather_frontend::home::Home;
use feather_frontend::playlist_search::PlayListSearch;
use feather_frontend::search_main::SearchMain;
use feather_frontend::statusbar::StatusBar;
use feather_frontend::userplaylist::UserPlayList;
use feather_frontend::{State, statusbar};
use feather_frontend::{
    backend::Backend, help::Help, history::History, player::SongPlayer, search::Search,
};
use ratatui::prelude::Alignment;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Padding;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::arch::x86_64::_mm256_castpd256_pd128;
use std::fs::OpenOptions;
use std::{env, sync::Arc};
use tokio::{
    sync::mpsc,
    time::{Duration, interval},
};

use log::{debug, info};
use simplelog::*;
use std::io::Write;

/// Entry point for the async runtime.
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();

    // Set up the logger to write to a file
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("app.log")
        .unwrap();

    // Initialize the logger
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::Config::default(),
        log_file,
    )
    .unwrap();

    let terminal = ratatui::init();
    let _app = App::new().render(terminal).await;
    ratatui::restore();
    Ok(())
}

/// Main application struct managing the state and UI components.
struct App<'a> {
    state: State,
    search: SearchMain<'a>,
    home: Home,
    history: History,
    help: Help,
    top_bar: TopBar,
    player: SongPlayer,
    status_bar: StatusBar,
    user_config: USERCONFIG,
    // backend: Arc<Backend>,
    help_mode: bool,
    exit: bool,
    prev_state: Option<State>,
    userplaylist: UserPlayList<'a>,
}

impl App<'_> {
    /// Creates a new instance of the application.
    fn new() -> Self {
        let history = Arc::new(HistoryDB::new().unwrap());
        let get_cookies = env::var("FEATHER_COOKIES").ok(); // Fetch cookies from environment variables if available.
        let (tx, rx) = mpsc::channel(32);
        let (tx_playlist_off, rx_playlist_off) = mpsc::channel(1);
        let (tx_playlist, rx_playlist) = mpsc::channel(500);
        let backend = Arc::new(
            Backend::new(history.clone(), get_cookies, tx.clone(), tx_playlist_off).unwrap(),
        );
        let search = Search::new(backend.clone());
        let playlist_search = PlayListSearch::new(backend.clone(), tx_playlist.clone());

        App {
            user_config: USERCONFIG::new().unwrap(), // unwrap because application should not be able to run without valid config
            state: State::Home,
            search: SearchMain::new(search, playlist_search),
            userplaylist: UserPlayList::new(backend.clone(), tx_playlist.clone()),
            history: History::new(history, backend.clone()),
            help: Help::new(),
            home: Home::new(),
            // current_playling_playlist: CurrentPlayingPlaylist {},
            top_bar: TopBar::new(),
            player: SongPlayer::new(backend.clone(), rx, rx_playlist, rx_playlist_off),
            // backend,
            help_mode: false,
            exit: false,
            status_bar: StatusBar::new(),
            prev_state: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(':') => {
                if let Ok(next_key) = crossterm::event::read() {
                    if let Event::Key(next_key) = next_key {
                        match next_key.code {
                            KeyCode::Char(';') => self.state = State::Home,
                            KeyCode::Char('s') => self.state = State::Search,
                            KeyCode::Char('u') => self.state = State::UserPlaylist,
                            KeyCode::Char('h') => self.state = State::History,
                            KeyCode::Char('p') => {
                                if self.state != State::SongPlayer {
                                    self.prev_state = Some(self.state);
                                }
                                self.state = State::SongPlayer;
                            }
                            KeyCode::Char('?') => {
                                self.help_mode = true;
                                self.state = State::HelpMode;
                            }
                            KeyCode::Char('q') => {
                                self.exit = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => self.handle_global_keystrokes(key),
        }
    }

    /// Handles global keystrokes and state transitions.
    fn handle_global_keystrokes(&mut self, key: KeyEvent) {
        match self.state {
            State::Search => match key.code {
                _ => self.search.handle_keystrokes(key),
            },
            State::HelpMode => match key.code {
                KeyCode::Esc => {
                    self.help_mode = false;
                }
                _ => (),
            },
            State::History => match key.code {
                _ => self.history.handle_keystrokes(key),
            },
            State::SongPlayer => match key.code {
                _ => self.player.handle_keystrokes(key),
            },
            State::UserPlaylist => match key.code {
                _ => self.userplaylist.handle_keystrokes(key),
            },
            _ => (),
        }
    }

    /// Main render loop for updating the UI.
    async fn render(mut self, mut terminal: DefaultTerminal) {
        let mut redraw_interval = interval(Duration::from_millis(250)); // Redraw every 250ms

        let bg_color = self.user_config.bg_color;
        let text_color = self.user_config.text_color;

        let global_style = Style::default()
            .fg(Color::Rgb(text_color.0, text_color.1, text_color.2))
            .bg(Color::Rgb(bg_color.0, bg_color.1, bg_color.2));

        while !self.exit {
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    let layout = Layout::default()
                        .direction(ratatui::layout::Direction::Vertical)
                        .constraints([
                            Constraint::Length(4),
                            Constraint::Min(0),
                            Constraint::Length(3),
                            Constraint::Length(2),
                        ])
                        .split(area);

                    self.top_bar
                        .render(layout[0], frame.buffer_mut(), &self.state);

                    // Background for the whole UI
                    frame.render_widget(Block::default().style(global_style), area);

                    if !self.help_mode {
                        match self.state {
                            State::Search => self.search.render(layout[1], frame.buffer_mut()),
                            State::History => self.history.render(layout[1], frame.buffer_mut()),
                            State::UserPlaylist => {
                                self.userplaylist.render(layout[1], frame.buffer_mut())
                            }
                            State::SongPlayer => {
                                if let Some(prev) = self.prev_state {
                                    match prev {
                                        State::Search => {
                                            self.search.render(layout[1], frame.buffer_mut())
                                        }
                                        State::History => {
                                            self.history.render(layout[1], frame.buffer_mut())
                                        }
                                        State::UserPlaylist => {
                                            self.userplaylist.render(layout[1], frame.buffer_mut());
                                        }
                                        _ => (),
                                    }
                                }
                            }
                            _ => (),
                        }
                        self.player.render(layout[2], frame.buffer_mut());
                        self.status_bar
                            .render(layout[3], frame.buffer_mut(), self.state);
                    } else {
                        self.help.render(layout[1], frame.buffer_mut());
                    }
                })
                .unwrap();

            tokio::select! {
                _ = redraw_interval.tick() => {}
                _ = async {
                    if poll(Duration::from_millis(100)).unwrap() {
                        if let Event::Key(key) = read().unwrap() {
                            self.handle_key(key);
                        }
                    }
                } => {}
            }
        }
    }
}

/// Represents the top bar UI component.
struct TopBar;

impl TopBar {
    fn new() -> Self {
        Self
    }
    fn render(&mut self, mut area: Rect, buf: &mut Buffer, state: &State) {
        let titles = ["Home", "Search", "History", "UserPlaylist"];

        // Add top padding by shifting the area down
        let top_padding = 1;
        area.y += top_padding;
        area.height = area.height.saturating_sub(top_padding);

        // Define colors
        let normal_style = Style::default().fg(Color::White);
        let selected_style = Style::default().fg(Color::Rgb(255, 255, 150)); // Light yellow

        let mut spans = vec![];

        for (i, title) in titles.iter().enumerate() {
            let style = match (i, state) {
                (0, State::Home) => selected_style,
                (1, State::Search) => selected_style,
                (2, State::History) => selected_style,
                (3, State::UserPlaylist) => selected_style,
                _ => normal_style,
            };

            spans.push(Span::styled(*title, style));

            if i < titles.len() - 1 {
                spans.push(Span::raw(" | ")); // Separator
            }
        }

        let text = Line::from(spans);
        let paragraph = Paragraph::new(text).alignment(Alignment::Left).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Feather v0.2")
                .title_alignment(Alignment::Center)
                .padding(Padding::new(1, 0, 0, 0)),
        );

        paragraph.render(area, buf);
    }
}
