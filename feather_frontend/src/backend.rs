#![allow(unused)]
use feather::database::SongError;
use feather::{
    ArtistName, SongId, SongName,
    database::{HistoryDB, HistoryEntry, Song, SongDatabase},
    player::{MpvError, Player},
    yt::YoutubeClient,
};
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
}

/// Represents a song with its name, ID, and artist(s).

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
}

impl Backend {
    /// Creates a new `Backend` instance.
    ///
    /// # Arguments
    /// * `history` - Shared reference to the history database.
    /// * `cookies` - Optional cookie string for authentication.
    ///
    /// # Returns
    /// * `Result<Self, BackendError>` - Returns `Backend` on success or an error on failure.
    pub fn new(
        history: Arc<HistoryDB>,
        cookies: Option<String>,
        tx: mpsc::Sender<bool>,
    ) -> Result<Self, BackendError> {
        Ok(Self {
            yt: YoutubeClient::new(),
            player: Player::new(cookies).map_err(BackendError::Mpv)?,
            history,
            song: Mutex::new(None),
            tx,
        })
    }

    pub fn loop_player(&self, is_loop: bool) -> Result<(),BackendError>{
        if is_loop{
            self.player.set_loop()?;
        }else{
            self.player.remove_loop()?;
        }
        Ok(())
    }

    /// Plays a song by fetching its URL from YouTube and passing it to the player.
    ///
    /// # Arguments
    /// * `song` - The song to be played.
    ///
    /// # Returns
    /// * `Result<(), BackendError>` - Returns `Ok(())` on success or an error on failure.
    pub async fn play_music(&self, song: Song, playlist_song: bool) -> Result<(), BackendError> {
        // println!("playing song");
        // const MAX_RETRIES: i32 = 8;
        // let id = song.id.to_string();

        // Fetch song URL with retry mechanism
        // let url = {
        //     let mut attempts = 0;
        //     loop {
        //         match self.yt.fetch_song_url(&id).await {
        //             Ok(url) => break url,
        //             Err(_) if attempts < MAX_RETRIES => {
        //                 attempts += 1;
        //                 tokio::time::sleep(Duration::from_millis(100)).await;
        //                 continue;
        //             }
        //             Err(e) => {
        //                 println!("failed to get url");
        //                 return Err(BackendError::YoutubeFetch(format!(
        //                     "Failed to fetch URL after {} attempts: {:?}",
        //                     MAX_RETRIES, e
        //                 )));
        //             }
        //         }
        //     }
        // };
        // println!("able to get url");

        // Update the currently playing song in a mutex-protected section

        // Play the song
        let url = format!("https://youtube.com/watch?v={}", song.id);
        self.player.play(&url).map_err(BackendError::Mpv)?;
        {
            let mut current_song = self
                .song
                .lock()
                .map_err(|e| BackendError::MutexPoisoned(e.to_string()))?;
            *current_song = Some(song.clone());
        }

        // Add the song to history
        self.history
            .add_entry(&HistoryEntry::from(song))
            .map_err(|e| BackendError::HistoryError(e.to_string()))?;
        self.loop_player(playlist_song)?;
        self.tx.send(playlist_song).await;

        Ok(())
    }
}
