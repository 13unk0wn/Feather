use crate::backend::{Backend, Song};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::{Alignment, Buffer, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task;

#[derive(PartialEq, PartialOrd, Debug)]
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
}

pub struct SongPlayer {
    backend: Arc<Backend>,            // Backend reference for controlling playback
    songstate: Arc<Mutex<SongState>>, // Current state of the player (Idle, Playing, etc.)
    song_playing: Arc<Mutex<Option<SongDetails>>>, // Details of the currently playing song
    rx: mpsc::Receiver<bool>,         // Receiver to listen for playback events
}

impl SongPlayer {
    pub fn new(backend: Arc<Backend>, rx: mpsc::Receiver<bool>) -> Self {
        let player = Self {
            backend,
            songstate: Arc::new(Mutex::new(SongState::Idle)),
            song_playing: Arc::new(Mutex::new(None)),
            rx,
        };
        player.observe_time(); // Start observing playback time
        player
    }

    // Function to continuously update the current playback time
    fn observe_time(&self) {
        let backend = Arc::clone(&self.backend);
        let song_playing = Arc::clone(&self.song_playing);

        tokio::task::spawn(async move {
            loop {
                // Try to get the current playback position from MPV
                match backend.player.player.get_property::<f64>("time-pos") {
                    Ok(time) => {
                        // Lock the song_playing mutex and update the current playback time
                        if let Ok(mut song_lock) = song_playing.lock() {
                            if let Some(song) = song_lock.as_mut() {
                                song.current_time = format!("{:.0}", time);
                            }
                        }
                    }
                    Err(_) => (), // Ignore errors (e.g., if MPV is not running)
                }

                tokio::time::sleep(Duration::from_millis(500)).await; // Update every 500ms
            }
        });
    }

    // Handle key presses for playback control
    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        if let Ok(state) = self.songstate.lock() {
            if *state == SongState::Playing {
                match key.code {
                    KeyCode::Char(' ') | KeyCode::Char(';') => {
                        // Toggle play/pause
                        if let Ok(_) = self.backend.player.play_pause() {};
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        // Seek forward
                        self.backend.player.seek_forward().ok();
                    }
                    KeyCode::Left | KeyCode::Char('j') => {
                        // Seek backward
                        self.backend.player.seek_backword().ok();
                    }
                    _ => (),
                };
            }
        }
    }

    // Function to check whether a song is playing
    fn check_playing(&mut self) {
        let songstate = Arc::clone(&self.songstate);
        let backend = Arc::clone(&self.backend);
        let song_playing = Arc::clone(&self.song_playing);

        task::spawn(async move {
            const MAX_IDLE_COUNT: i32 = 5; // Max checks before considering it an error
            let mut idle_count = 0;

            // Initial delay before checking playback status
            tokio::time::sleep(Duration::from_secs(1)).await;

            loop {
                match backend.player.is_playing() {
                    Ok(true) => {
                        if let Ok(mut state) = songstate.lock() {
                            if let Ok(mut song_lock) = song_playing.lock() {
                                if let Ok(song) = backend.song.lock() {
                                    if let Some(value) = song.as_ref() {
                                        let total_duration = backend
                                            .player
                                            .duration()
                                            .parse::<f64>()
                                            .map(|d| {
                                                let total = d as i64;
                                                format!("{:02}:{:02}", total / 60, total % 60)
                                            })
                                            .unwrap_or_default();
                                        *song_lock = Some(SongDetails {
                                            song: value.clone(),
                                            current_time: backend.player.get_current_time(),
                                            total_duration,
                                        });
                                        *state = SongState::Playing;
                                        return; // Exit once playing is confirmed
                                    }
                                }
                            }
                        }
                        idle_count = 0; // Reset idle count since the song is playing
                    }
                    Ok(false) => {
                        // Song is not playing, set state to Idle
                        if let Ok(mut state) = songstate.lock() {
                            *state = SongState::Idle;
                        }
                        idle_count += 1;
                    }
                    Err(_) => idle_count += 1, // Increase idle count if an error occurs
                }

                // If too many idle checks, assume an error occurred
                if idle_count >= MAX_IDLE_COUNT {
                    if let Ok(mut state) = songstate.lock() {
                        if *state == SongState::Loading {
                            *state = SongState::ErrorPlayingoSong;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await; // Check every 2 seconds
            }
        });
    }

    // Render the player UI
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Check for playback event signals
        if self.rx.try_recv().is_ok() {
            if let Ok(mut state) = self.songstate.lock() {
                *state = SongState::Loading;
            }
            self.check_playing(); // Start checking for playback status
        }

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        if let Ok(state) = self.songstate.lock() {
            let text = match *state {
                SongState::Idle => vec![Line::from("No song is playing")],
                SongState::Playing => {
                    if let Ok(song_playing) = self.song_playing.lock() {
                        song_playing.as_ref().map_or_else(
                            || vec![Line::from("Loading...")],
                            |song| {
                                let current_time = song
                                    .current_time
                                    .parse::<i64>()
                                    .map(|t| format!("{:02}:{:02}", t / 60, t % 60))
                                    .unwrap_or_default();
                                vec![
                                    Line::from(Span::styled(
                                        song.song.song_name.clone(),
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
                    vec![Line::from("Loading Song")]
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
