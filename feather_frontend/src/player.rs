#![allow(unused)]
use crate::backend::Backend;
use crate::playlist_search;
use crossterm::event::{KeyCode, KeyEvent};
use feather::database::{Song, SongDatabase};
use log::{debug, error, info};
use ratatui::prelude::{Alignment, Buffer, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
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
}

pub struct SongPlayer {
    backend: Arc<Backend>,            // Backend reference for controlling playback
    songstate: Arc<Mutex<SongState>>, // Current state of the player (Idle, Playing, etc.)
    song_playing: Arc<Mutex<Option<SongDetails>>>, // Details of the currently playing song
    rx: mpsc::Receiver<bool>,         // Receiver to listen for playback events
    is_playlist: Arc<Mutex<bool>>,
    rx_playlist_off: mpsc::Receiver<bool>,
}

impl SongPlayer {
    pub fn new(
        backend: Arc<Backend>,
        rx: mpsc::Receiver<bool>,
        _rx_playlist: mpsc::Receiver<Arc<Mutex<SongDatabase>>>,
        rx_playlist_off: mpsc::Receiver<bool>,
    ) -> Self {
        let player = Self {
            backend,
            songstate: Arc::new(Mutex::new(SongState::Idle)),
            song_playing: Arc::new(Mutex::new(None)),
            rx,
            is_playlist: Arc::new(Mutex::new(false)),
            rx_playlist_off,
        };
        player.observe_time(); // Start observing playback time
        player.observe_song_end(); // Start observing song end for playlists
        player
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
                        self.backend.player.high_volume().is_ok();
                    }
                    KeyCode::Down => {
                        self.backend.player.low_volume().is_ok();
                    }
                    KeyCode::Char(' ') | KeyCode::Char(';') => {
                        if let Ok(_) = self.backend.player.play_pause() {};
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

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        if let Ok(state) = self.songstate.lock() {
            let text = match *state {
                SongState::Idle => vec![Line::from("No song is playing")],
                SongState::Playing => {
                    if let Ok(mut song_playing) = self.song_playing.lock() {
                        song_playing.as_mut().map_or_else(
                            || vec![Line::from("Loading...")],
                            |song| {
                                if song.tries < 3 && song.total_duration == "00::00" {
                                    song.total_duration = self.backend.player.duration();
                                    song.tries += 1;
                                }
                                let current_time = song
                                    .current_time
                                    .parse::<i64>()
                                    .map(|t| format!("{:02}:{:02}", t / 60, t % 60))
                                    .unwrap_or_default();
                                vec![
                                    Line::from(Span::styled(
                                        song.song.title.clone(),
                                        Style::default().add_modifier(Modifier::BOLD),
                                    )),
                                    Line::from(format!("{}/{}", current_time, song.total_duration)),
                                ]
                            },
                        )
                    } else {
                        vec![Line::from("Error accessing song details")]
                    }
                }
                SongState::Loading => {
                    vec![Line::from("Loading...")]
                }
                SongState::ErrorPlayingoSong => {
                    vec![Line::from("Error Playing Song")]
                }
            };
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .render(inner, buf);
        }
    }
}
