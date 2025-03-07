#![allow(unused)]
use crate::backend::Backend;
use crate::playlist_search;
use crossterm::event::{KeyCode, KeyEvent};
use feather::database::{Song, SongDatabase};
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
    tries: usize,
}

pub struct SongPlayer {
    backend: Arc<Backend>,            // Backend reference for controlling playback
    songstate: Arc<Mutex<SongState>>, // Current state of the player (Idle, Playing, etc.)
    song_playing: Arc<Mutex<Option<SongDetails>>>, // Details of the currently playing song
    rx: mpsc::Receiver<bool>,         // Receiver to listen for playback events
    playlist: Option<Arc<Mutex<SongDatabase>>>,
    current_index_playlist: Arc<Mutex<usize>>,
    is_playlist: bool,
    rx_playlist :  mpsc::Receiver<Arc<Mutex<SongDatabase>>>,
}

impl SongPlayer {
    pub fn new(backend: Arc<Backend>, rx: mpsc::Receiver<bool>,rx_playlist :  mpsc::Receiver<Arc<Mutex<SongDatabase>>>) -> Self {
        let player = Self {
            backend,
            songstate: Arc::new(Mutex::new(SongState::Idle)),
            song_playing: Arc::new(Mutex::new(None)),
            rx,
            playlist: None,
            is_playlist: false,
            current_index_playlist: Arc::new(Mutex::new(0)),
            rx_playlist,
        };
        player.observe_time(); // Start observing playback time
        player
    }

   

    // Function to continuously update the current playback time
    fn observe_time(&self) {
        let backend = Arc::clone(&self.backend);
        let song_playing = Arc::clone(&self.song_playing);

        tokio::task::spawn(async move {
            let _ = tokio::time::sleep(Duration::from_secs(2)).await;
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

    fn check_playing(&mut self) {
    let songstate = Arc::clone(&self.songstate);
    let backend = Arc::clone(&self.backend);
    let playlist = self.playlist.clone();
    let current_index = Arc::clone(&self.current_index_playlist); // Clone it

    task::spawn(async move {
        const MAX_IDLE_COUNT: i32 = 5;
        let mut idle_count = 0;

        tokio::time::sleep(Duration::from_secs(1)).await;

        loop {
            match backend.player.is_playing() {
                Ok(true) => {
                    idle_count = 0;
                }
                Ok(false) | Err(_) => {
                    idle_count += 1;
                }
            }

            if idle_count >= MAX_IDLE_COUNT {
                if let Some(playlist) = &playlist {
                    let len = playlist.lock().unwrap().db.len();

                    let mut index = current_index.lock().unwrap();
                    *index = (*index + 1) % len; // Update index

                    drop(index); // Explicitly drop to avoid deadlock
                }

                if let Ok(mut state) = songstate.lock() {
                    *state = SongState::Idle;
                }

                // Play next song
                if let Some(playlist) = &playlist {
                    let playlist = playlist.lock().unwrap();
                    let new_index = *current_index.lock().unwrap();
                    if let Ok(song) = playlist.get_song_by_index(new_index) {
                        let backend = backend.clone();
                        tokio::spawn(async move {
                            backend.play_music(song, true).await;
                        });
                    }
                }
                return;
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}


    // Render the player UI
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Check for playback event signals
        if let Ok(is_playlist) = self.rx.try_recv() {
            if is_playlist {
                self.is_playlist = true;
            } else {
                let _ = self.playlist.take();
                self.is_playlist = false;
            }
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