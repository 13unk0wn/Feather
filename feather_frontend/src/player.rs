#![allow(unused)]
use crate::backend::Backend;
use crate::config::USERCONFIG;
use crate::playlist_search;
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent};
use feather::database::{Song, SongDatabase};
use log::{debug, error, info};
use ratatui::layout::{Constraint, Layout};
use ratatui::prelude::Direction;
use ratatui::prelude::Stylize;
use ratatui::prelude::{Alignment, Buffer, Rect};
use ratatui::style::Color;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::widgets::{BorderType, Gauge};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task;

#[derive(PartialEq, PartialOrd, Debug, Clone)]
enum SongState {
    Idle,              // No song is playing
    Playing,           // A song is currently playing
    Loading,           // Song is loading
    ErrorPlayingoSong, // An error occurred while playing the song
}

#[derive(Clone)]
pub struct SongDetails {
    song: Song,             // Information about the song
    current_time: String,   // Current playback time (formatted as MM:SS)
    total_duration: String, // Total duration of the song
    tries: usize,
    current_volume: i64,
    pause: bool,
}

pub struct SongPlayer {
    backend: Arc<Backend>,            // Backend reference for controlling playback
    songstate: Arc<Mutex<SongState>>, // Current state of the player (Idle, Playing, etc.)
    song_playing: Arc<Mutex<Option<SongDetails>>>, // Details of the currently playing song
    rx: mpsc::Receiver<bool>,         // Receiver to listen for playback events
    is_playlist: Arc<Mutex<bool>>,
    rx_playlist_off: mpsc::Receiver<bool>,
    config: Rc<USERCONFIG>,
}

impl SongPlayer {
    pub fn new(
        backend: Arc<Backend>,
        rx: mpsc::Receiver<bool>,
        _rx_playlist: mpsc::Receiver<Arc<Mutex<SongDatabase>>>,
        rx_playlist_off: mpsc::Receiver<bool>,
        config: Rc<USERCONFIG>,
    ) -> Self {
        let player = Self {
            backend,
            songstate: Arc::new(Mutex::new(SongState::Idle)),
            song_playing: Arc::new(Mutex::new(None)),
            rx,
            is_playlist: Arc::new(Mutex::new(false)),
            rx_playlist_off,
            config,
        };
        player.observe_time(); // Start observing playback time
        player.add_time();
        player.observe_song_end(); // Start observing song end for playlists
        player
    }

    fn add_time(&self) {
        let backend = self.backend.clone();

        tokio::task::spawn(async move {
            loop {
                if backend.player.is_playing().unwrap_or(false) {
                    debug!("Adding time");
                    backend.user_profile.add_time();
                    tokio::time::sleep(Duration::from_secs(1)).await;
                } else {
                    debug!("not adding time");
                }
            }
        });
    }

    fn observe_time(&self) {
        let backend = Arc::clone(&self.backend);
        let song_playing = Arc::clone(&self.song_playing);

        tokio::task::spawn(async move {
            let _ = tokio::time::sleep(Duration::from_secs(2)).await;
            loop {
                match backend.player.player.get_property::<f64>("time-pos") {
                    Ok(time) => {
                        if let Ok(mut song_lock) = song_playing.lock() {
                            if let Some(song) = song_lock.as_mut() {
                                song.current_time = format!("{:.0}", time);
                            }
                        }
                    }
                    Err(_) => (), // Ignore errors (e.g., if MPV is not running)
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    // Modified observe_song_end without relying on duration
    fn observe_song_end(&self) {
        let backend = Arc::clone(&self.backend);
        let songstate = Arc::clone(&self.songstate);
        let is_playlist = self.is_playlist.clone();

        tokio::task::spawn(async move {
            let mut was_playing = true;
            let mut idle_count = 0;
            const MAX_IDLE_COUNT: i32 = 3; // Number of seconds to wait before considering song ended

            loop {
                let mut m_playlist = false;
                if let Ok(playlist) = is_playlist.lock() {
                    m_playlist = *playlist;
                    // info!("Is this playlist  :  {playlist}");
                }
                if m_playlist {
                    let is_playing = backend.player.is_playing().unwrap_or(false);

                    // info!("{} {}", was_playing, is_playing);
                    if is_playing {
                        was_playing = true;
                        idle_count = 0;
                    } else if was_playing && !is_playing {
                        idle_count += 1;
                        if idle_count >= MAX_IDLE_COUNT {
                            let should_play_next = if let Ok(state) = songstate.lock() {
                                *state == SongState::Playing || *state == SongState::Idle
                            } else {
                                false
                            };

                            if should_play_next {
                                backend.next_song_playlist().await;
                                was_playing = false; // Reset after playing next song
                                idle_count = 0;
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await; // Check every second
            }
        });
    }

    fn check_playing(&mut self) {
        let songstate = Arc::clone(&self.songstate);
        let backend = Arc::clone(&self.backend);
        let song_playing = Arc::clone(&self.song_playing);

        let mut current_state = if let Ok(state) = songstate.lock() {
            state.clone()
        } else {
            SongState::Idle
        };

        task::spawn(async move {
            const MAX_IDLE_COUNT: i32 = 10;
            let mut idle_count = 0;

            tokio::time::sleep(Duration::from_secs(15)).await;

            loop {
                let is_playing = match backend.player.is_playing() {
                    Ok(playing) => playing,
                    Err(_) => false,
                };

                if is_playing {
                    idle_count = 0;
                    if current_state != SongState::Playing {
                        if let Ok(mut state) = songstate.lock() {
                            *state = SongState::Playing;
                            current_state = SongState::Playing;
                        }

                        if let Ok(mut song_details) = song_playing.lock() {
                            if let Some(current_song) = backend.song.lock().unwrap().as_ref() {
                                let duration = backend.player.duration().parse::<u64>().unwrap();
                                *song_details = Some(SongDetails {
                                    song: current_song.clone(),
                                    current_time: "0".to_string(),
                                    total_duration: format!(
                                        "{:02}:{:02}",
                                        duration / 60,
                                        duration % 60
                                    ),
                                    current_volume: backend.player.current_volume().unwrap_or(0),
                                    pause: backend.player.is_playing().unwrap_or(false),
                                    tries: 0,
                                });
                            }
                        }
                    }
                } else {
                    idle_count += 1;
                    if idle_count >= MAX_IDLE_COUNT {
                        if let Ok(mut state) = songstate.lock() {
                            *state = SongState::Idle;
                            current_state = SongState::Idle;

                            if let Ok(mut song_details) = song_playing.lock() {
                                *song_details = None;
                            }
                        }
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_secs(4)).await;
            }
        });
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        if let Ok(state) = self.songstate.lock() {
            if *state == SongState::Playing {
                match key.code {
                    KeyCode::Char('n') => {
                        if let Ok(is_playlist) = self.is_playlist.lock() {
                            if *is_playlist {
                                drop(is_playlist);
                                let backend = self.backend.clone();
                                tokio::spawn(async move {
                                    backend.next_song_playlist().await;
                                });
                            }
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Ok(is_playlist) = self.is_playlist.lock() {
                            if *is_playlist {
                                drop(is_playlist);
                                let backend = self.backend.clone();
                                tokio::spawn(async move {
                                    backend.prev_song_playlist().await;
                                });
                            }
                        }
                    }
                    KeyCode::Up => {
                        if self.backend.player.high_volume().is_ok() {
                            if let Ok(mut song_details) = self.song_playing.lock() {
                                if let Some(song) = song_details.as_mut() {
                                    song.current_volume =
                                        self.backend.player.current_volume().unwrap_or(0);
                                    debug!("{}", song.current_volume);
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        if self.backend.player.low_volume().is_ok() {
                            if let Ok(mut song_details) = self.song_playing.lock() {
                                if let Some(song) = song_details.as_mut() {
                                    song.current_volume =
                                        self.backend.player.current_volume().unwrap_or(0);
                                }
                            }
                        }
                    }
                    KeyCode::Char(' ') | KeyCode::Char(';') => {
                        if let Ok(_) = self.backend.player.play_pause() {
                            if let Ok(mut song_details) = self.song_playing.lock() {
                                if let Some(song) = song_details.as_mut() {
                                    song.pause = !song.pause;
                                }
                            }
                        }
                    }

                    KeyCode::Right | KeyCode::Char('l') => {
                        self.backend.player.seek_forward().ok();
                    }
                    KeyCode::Left | KeyCode::Char('j') => {
                        self.backend.player.seek_backword().ok();
                    }
                    _ => (),
                };
            }
        }
    }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if let Ok(value) = self.rx_playlist_off.try_recv() {
            if let Ok(mut playlist) = self.is_playlist.lock() {
                *playlist = false;
            }
        }
        if let Ok(is_playlist) = self.rx.try_recv() {
            if let Ok(mut playlist) = self.is_playlist.lock() {
                if is_playlist {
                    *playlist = true;
                } else {
                    let _ = self.backend.playlist.lock().unwrap().take();
                    *playlist = false;
                }
            }
            if let Ok(mut state) = self.songstate.lock() {
                *state = SongState::Loading;
            }
            self.check_playing();
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(10), // Pause/Player
                Constraint::Min(0),     // Player
                Constraint::Length(20), // Volume
            ])
            .split(area);

        let mut title = None;
        let mut percentage = 0.0;
        let mut volume = 0;
        let mut text = vec![Line::from("")];
        let mut pause = false;
        let progress_bar_color = self.config.player_progress_bar_color;

        if let Ok(state) = self.songstate.lock() {
            text = match *state {
                SongState::Idle => vec![Line::from("No song is playing")],
                SongState::Playing => {
                    if let Ok(mut song_playing) = self.song_playing.lock() {
                        song_playing.as_mut().map_or_else(
                            || vec![Line::from("Loading...")],
                            |song| {
                                if song.tries < 3 && song.total_duration == "00:00" {
                                    song.total_duration = self.backend.player.duration();
                                    song.tries += 1;
                                }
                                title = Some(song.song.title.clone());
                                volume = song.current_volume;
                                pause = song.pause;

                                let current_time_secs = song
                                    .current_time
                                    .split(':')
                                    .filter_map(|s| s.parse::<i64>().ok())
                                    .reduce(|acc, x| acc * 60 + x)
                                    .unwrap_or(0);

                                let total_time_secs = song
                                    .total_duration
                                    .split(':')
                                    .filter_map(|s| s.parse::<i64>().ok())
                                    .reduce(|acc, x| acc * 60 + x)
                                    .unwrap_or(1);

                                percentage = current_time_secs as f64 / total_time_secs as f64;

                                let current_time = format!(
                                    "{:02}:{:02}",
                                    current_time_secs / 60,
                                    current_time_secs % 60
                                );
                                vec![Line::from(format!(
                                    "{}/{}",
                                    current_time, song.total_duration
                                ))]
                            },
                        )
                    } else {
                        vec![Line::from("Error accessing song details")]
                    }
                }
                SongState::Loading => vec![Line::from("Loading...")],
                SongState::ErrorPlayingoSong => vec![Line::from("Error Playing Song")],
            };

            match *state {
                SongState::Playing => {
                    if let Some(title) = title {
                        let block = Block::default()
                            .borders(Borders::ALL)
                            .title(title)
                            .title_alignment(Alignment::Center)
                            .border_type(BorderType::Rounded);

                        let label_text =
                            text.get(0).map(|line| line.to_string()).unwrap_or_default();

                        let gauge = Gauge::default()
                            .block(block)
                            .gauge_style(Style::default().fg(Color::Rgb(
                                progress_bar_color.0,
                                progress_bar_color.1,
                                progress_bar_color.2,
                            )))
                            .ratio(percentage.min(1.0))
                            .label(Span::styled(label_text, Style::default().fg(Color::Blue)));

                        gauge.render(chunks[1], buf);
                    }
                }
                SongState::ErrorPlayingoSong | SongState::Loading | SongState::Idle => {
                    let border = Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded);

                    let inner_area = border.inner(chunks[1]);
                    border.render(chunks[1], buf);

                    Paragraph::new(text)
                        .alignment(Alignment::Center)
                        .render(inner_area, buf);
                }
            }
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let inner_block = block.inner(chunks[0]);
        block.render(chunks[0], buf);
        let icon = if pause {
            self.config.pause_icon.clone()
        } else {
            self.config.play_icon.clone()
        };
        let mut text = Paragraph::new(icon)
            .alignment(Alignment::Center)
            .render(inner_block, buf);
        let volume_color = self.config.player_volume_bar_color;
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Volume")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);
        let gauge = Gauge::default()
            .block(block)
            .gauge_style(Style::default().fg(Color::Rgb(
                volume_color.0,
                volume_color.1,
                volume_color.2,
            )))
            .ratio(((volume as f64) / 100.0).min(1.0))
            .label(Span::styled(
                format!("{}", volume),
                Style::default().fg(Color::Blue),
            ));
        gauge.render(chunks[2], buf);
    }
}
