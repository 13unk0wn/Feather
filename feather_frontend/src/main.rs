use color_eyre::eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, poll, read};
use feather::database::HistoryDB;
use feather_frontend::{backend::Backend, history::History, player::SongPlayer, search::Search};
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Widget},
};
use std::{env, sync::Arc};
use tokio::{
    sync::mpsc,
    time::{Duration, interval},
};

/// Entry point for the async runtime.
#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    let terminal = ratatui::init();
    let _app = App::new().render(terminal).await;
    ratatui::restore();
    Ok(())
}

/// Enum representing different states of the application.
#[derive(Debug)]
enum State {
    HelpMode,
    Global,
    Search,
    History,
    // UserPlaylist,
    // CurrentPlayingPlaylist,
    SongPlayer,
}

/// Main application struct managing the state and UI components.
struct App<'a> {
    state: State,
    search: Search<'a>,
    history: History,
    // user_playlist: UserPlaylist,
    // current_playling_playlist: CurrentPlayingPlaylist,
    top_bar: TopBar,
    player: SongPlayer,
    // backend: Arc<Backend>,
    help_mode: bool,
    exit: bool,
}

impl App<'_> {
    /// Creates a new instance of the application.
    fn new() -> Self {
        let history = Arc::new(HistoryDB::new().unwrap());
        let get_cookies = env::var("FEATHER_COOKIES").ok(); // Fetch cookies from environment variables if available.
        let backend = Arc::new(Backend::new(history.clone(), get_cookies).unwrap());
        let (tx, rx) = mpsc::channel(32);

        App {
            state: State::Global,
            search: Search::new(backend.clone(), tx.clone()),
            history: History::new(history, backend.clone(), tx.clone()),
            // user_playlist: UserPlaylist {},
            // current_playling_playlist: CurrentPlayingPlaylist {},
            top_bar: TopBar::new(),
            player: SongPlayer::new(backend.clone(), rx),
            // backend,
            help_mode: false,
            exit: false,
        }
    }

    /// Handles global keystrokes and state transitions.
    fn handle_global_keystrokes(&mut self, key: KeyEvent) {
        match self.state {
            State::Global => match key.code {
                KeyCode::Char('s') => self.state = State::Search,
                KeyCode::Char('h') => self.state = State::History,
                KeyCode::Char('p') => self.state = State::SongPlayer,
                KeyCode::Char('?') => {
                    self.help_mode = true;
                    self.state = State::HelpMode;
                }
                KeyCode::Esc => {
                    self.exit = true;
                }
                _ => (),
            },
            State::Search => match key.code {
                KeyCode::Esc => self.state = State::Global,
                _ => self.search.handle_keystrokes(key),
            },
            State::HelpMode => match key.code {
                KeyCode::Esc => {
                    self.state = State::Global;
                    self.help_mode = false;
                }
                _ => (),
            },
            State::History => match key.code {
                KeyCode::Esc => self.state = State::Global,
                _ => self.history.handle_keystrokes(key),
            },
            State::SongPlayer => match key.code {
                KeyCode::Esc => self.state = State::Global,
                _ => self.player.handle_keystrokes(key),
            },
        }
    }

    /// Main render loop for updating the UI.
    async fn render(mut self, mut terminal: DefaultTerminal) {
        let mut redraw_interval = interval(Duration::from_millis(250)); // Redraw every 250ms

        while !self.exit {
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    let layout = Layout::default()
                        .direction(ratatui::layout::Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(10),
                            Constraint::Percentage(75),
                            Constraint::Percentage(15),
                        ])
                        .split(area);

                    let middle_layout = Layout::default()
                        .direction(ratatui::layout::Direction::Horizontal)
                        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(layout[1]);

                    if !self.help_mode {
                        self.top_bar
                            .render(layout[0], frame.buffer_mut(), &self.state);
                        self.search.render(middle_layout[0], frame.buffer_mut());
                        self.history.render(middle_layout[1], frame.buffer_mut());
                        self.player.render(layout[2], frame.buffer_mut());
                    } else {
                        let rows = vec![
                            Row::new(vec![Cell::from("s"), Cell::from("Search")]),
                            Row::new(vec![Cell::from("h"), Cell::from("History")]),
                            Row::new(vec![Cell::from("p"), Cell::from("Player")]),
                            Row::new(vec![Cell::from("?"), Cell::from("Toggle Help Mode")]),
                            Row::new(vec![
                                Cell::from("TAB (Search)"),
                                Cell::from("Toggle between search input and results"),
                            ]),
                            Row::new(vec![
                                Cell::from("Esc (Global)"),
                                Cell::from("Quit application"),
                            ]),
                            Row::new(vec![
                                Cell::from("Esc (Non-Global)"),
                                Cell::from("Switch to Global Mode"),
                            ]),
                            Row::new(vec![
                                Cell::from("↑ / k(History/Search)"),
                                Cell::from("Navigate up in list"),
                            ]),
                            Row::new(vec![
                                Cell::from("↓ / j(History/Search)"),
                                Cell::from("Navigate down in list"),
                            ]),
                            Row::new(vec![
                                Cell::from("Space / ; (Player)"),
                                Cell::from("Pause current song"),
                            ]),
                            Row::new(vec![
                                Cell::from("→ (Player)"),
                                Cell::from("Skip forward 5 seconds"),
                            ]),
                            Row::new(vec![
                                Cell::from("← (Player)"),
                                Cell::from("Rewind 5 seconds"),
                            ]),
                        ];

                        let help_table = Table::new(
                            rows,
                            [Constraint::Percentage(20), Constraint::Percentage(80)],
                        )
                        .block(Block::default().borders(Borders::ALL).title("Help"))
                        .header(Row::new(vec![Cell::from("Key"), Cell::from("Action")]));

                        help_table.render(area, frame.buffer_mut());
                    }
                })
                .unwrap();

            tokio::select! {
                _ = redraw_interval.tick() => {}
                _ = async {
                    if poll(Duration::from_millis(100)).unwrap() {
                        if let Event::Key(key) = read().unwrap() {
                            self.handle_global_keystrokes(key);
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
    fn render(&mut self, area: Rect, buf: &mut Buffer, state: &State) {
        let s = format!("Feather | Current Mode : {:?}", state);
        Paragraph::new(s)
            .block(Block::default().borders(Borders::ALL))
            .render(area, buf);
    }
}

#[allow(unused)]
/// Placeholder struct for user playlists.
struct UserPlaylist {}
#[allow(unused)]
/// Placeholder struct for currently playing playlist.
struct CurrentPlayingPlaylist {}
