#![allow(non_snake_case)]
#![allow(unused)]
use feather::database::PlaylistManager;
use feather::database::{PlaylistManagerError, SongError};
use feather::{
    ArtistName, SongId, SongName,
    database::{HistoryDB, HistoryEntry, Song, SongDatabase},
    player::{MpvError, Player},
    yt::YoutubeClient,
};
use log::debug;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::mpsc;

use thiserror::Error;

/// The `Backend` struct manages the YouTube client, music player, and history database.
/// It also tracks the currently playing song.
pub struct Backend {
    pub yt: YoutubeClient,         // YouTube client for fetching song URLs
    pub player: Player,            // Music player instance
    pub history: Arc<HistoryDB>,   // Shared history database
    pub song: Mutex<Option<Song>>, // Mutex-protected optional current song
    tx: mpsc::Sender<bool>,
    pub playlist: Arc<Mutex<Option<SongDatabase>>>,
    current_index_playlist: Arc<Mutex<usize>>,
    pub PlayListManager: Arc<PlaylistManager>,
    tx_playlist_off : mpsc::Sender<bool>,
}

/// Defines possible errors that can occur in the `Backend`.
#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Player error: {0}")]
    Mpv(#[from] MpvError), // Error related to the music player

    #[error("Failed to fetch YouTube URL")]
    YoutubeFetch(String), // Error when fetching a song URL from YouTube

    #[error("Mutex poisoned: {0}")]
    MutexPoisoned(String), // Error when accessing a poisoned mutex

    #[error("History database error: {0}")]
    HistoryError(String), // Error related to history database operations

    #[error("Playback error: {0}")]
    PlaybackError(String), // Error related to playback issues

    #[error("Playlist error : {0}")]
    PlaylistError(#[from] SongError),

    #[error("UserPlayListError : {0}")]
    UserPlayListError(#[from] PlaylistManagerError),
}

impl Backend {
    /// Creates a new `Backend` instance.
    pub  fn new(
        history: Arc<HistoryDB>,
        cookies: Option<String>,
        tx: mpsc::Sender<bool>,
        tx_playlist_off : mpsc::Sender<bool>
    ) -> Result<Self, BackendError> {
        Ok(Self {
            current_index_playlist: Arc::new(Mutex::new(0)),
            playlist: Arc::new(Mutex::new(None)),
            yt: YoutubeClient::new(),
            player: Player::new(cookies).map_err(BackendError::Mpv)?,
            history,
            song: Mutex::new(None),
            tx,
            PlayListManager: Arc::new(PlaylistManager::new()?),
            tx_playlist_off,
        })
    }

    pub async fn drop_playlist(&self) -> Result<(), BackendError> {
        if let Ok(mut playlist) = self.playlist.lock() {
            *playlist = None;
        }
       self.tx_playlist_off.send(false).await;
        Ok(())
    }

    /// Plays a playlist starting at the given index.
    pub async fn play_playlist(&self, song_db: SongDatabase, index: usize) {
        // Step 1: Update the playlist
        {
            let mut playlist = self.playlist.lock().expect("Failed to lock playlist");
            *playlist = Some(song_db);
        }

        // Step 2: Extract the song to play
        let song_to_play = {
            let playlist = self.playlist.lock().expect("Failed to lock playlist");
            if let Some(playlist) = playlist.as_ref() {
                playlist.get_song_by_index(index).ok()
            } else {
                None
            }
        };

        // Step 3: Play the song and update index
        if let Some(song) = song_to_play {
            self.play_music(song, true).await;
            let mut i = self
                .current_index_playlist
                .lock()
                .expect("Failed to lock index");
            *i = index;
        }
    }

    /// Advances to the next song in the playlist.
    pub async fn next_song_playlist(&self) {
        // println!("Recieved request");
        let (song_to_play, new_index) = {
            let playlist = self.playlist.lock().expect("Failed to lock playlist");
            if let Some(playlist) = playlist.as_ref() {
                let len = playlist.db.len();
                let mut current_index = self
                    .current_index_playlist
                    .lock()
                    .expect("Failed to lock index");
                *current_index += 1;
                *current_index %= len;
                let song = playlist.get_song_by_index(*current_index).ok();
                (song, *current_index)
            } else {
                (None, 0)
            }
        };

        if let Some(song) = song_to_play {
            self.play_music(song, true).await;
        }
    }

    /// Goes back to the previous song in the playlist.
    pub async fn prev_song_playlist(&self) {
        let (song_to_play, new_index) = {
            let playlist = self.playlist.lock().expect("Failed to lock playlist");
            if let Some(playlist) = playlist.as_ref() {
                let mut current_index = self
                    .current_index_playlist
                    .lock()
                    .expect("Failed to lock index");
                if *current_index > 0 {
                    *current_index -= 1;
                }
                let song = playlist.get_song_by_index(*current_index).ok();
                (song, *current_index)
            } else {
                (None, 0)
            }
        };

        if let Some(song) = song_to_play {
            self.play_music(song, true).await;
        }
    }

    /// Sets or removes looping for the player.
    pub fn loop_player(&self, is_loop: bool) -> Result<(), BackendError> {
        if is_loop {
            self.player.set_loop()?;
        } else {
            self.player.remove_loop()?;
        }
        Ok(())
    }

    /// Plays a song by fetching its URL and updating history.
    pub async fn play_music(&self, song: Song, playlist_song: bool) -> Result<(), BackendError> {
        let url = format!("https://youtube.com/watch?v={}", song.id);
        self.player.play(&url).map_err(BackendError::Mpv)?;

        // Update current song
        {
            let mut current_song = self
                .song
                .lock()
                .map_err(|e| BackendError::MutexPoisoned(e.to_string()))?;
            *current_song = Some(song.clone());
        }

        // Add to history
        self.history
            .add_entry(&HistoryEntry::from(song))
            .map_err(|e| BackendError::HistoryError(e.to_string()))?;

        self.loop_player(!playlist_song)?;
        self.tx.send(playlist_song).await;

        Ok(())
    }
}
