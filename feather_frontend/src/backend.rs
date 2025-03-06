#![allow(unused)]
use feather::database::SongError;
use feather::{
    ArtistName, SongId, SongName,
    database::{HistoryDB, HistoryEntry, SongDatabase,Song},
    player::{MpvError, Player},
    yt::YoutubeClient,
};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use thiserror::Error;

/// The `Backend` struct manages the YouTube client, music player, and history database.
/// It also tracks the currently playing song.
pub struct Backend {
    pub yt: YoutubeClient,         // YouTube client for fetching song URLs
    pub player: Player,            // Music player instance
    pub history: Arc<HistoryDB>,   // Shared history database
    pub song: Mutex<Option<Song>>, // Mutex-protected optional current song
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
    PlaylistError(#[from] SongError)
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
    pub fn new(history: Arc<HistoryDB>, cookies: Option<String>) -> Result<Self, BackendError> {
        Ok(Self {
            yt: YoutubeClient::new(cookies.clone()),
            player: Player::new(cookies).map_err(BackendError::Mpv)?,
            history,
            song: Mutex::new(None),
        })
    }

    async fn play_playlist(
    &self,
    current_playlist: Arc<Mutex<Option<SongDatabase>>>,
    selected_song: Option<usize>,
) -> Result<(), BackendError> {
    let selected_id = selected_song.unwrap_or(0);
    
    // Lock the playlist to get the song
    if let Ok(mut playlist) = current_playlist.lock() {
        if let Some(p) = playlist.take() { // Take the playlist out of the Option
            // Get the song, and release the lock here
            let song = p.get_song_by_index(selected_id)?;
            
            // Now that we have the song, release the lock and play music asynchronously
            drop(playlist); // Explicitly drop the lock here
            
            // Play the song
            self.play_music(song).await?;
        }
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
    pub async fn play_music(&self, song: Song) -> Result<(), BackendError> {
        println!("playing song");
        const MAX_RETRIES: i32 = 8;
        let id = song.id.to_string();

        // Fetch song URL with retry mechanism
        let url = {
            let mut attempts = 0;
            loop {
                match self.yt.fetch_song_url(&id).await {
                    Ok(url) => break url,
                    Err(_) if attempts < MAX_RETRIES => {
                        attempts += 1;
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    Err(e) => {
                        println!("failed to get url");
                        return Err(BackendError::YoutubeFetch(format!(
                            "Failed to fetch URL after {} attempts: {:?}",
                            MAX_RETRIES, e
                        )));
                    }
                }
            }
        };
        println!("able to get url");

        // Update the currently playing song in a mutex-protected section
        {
            let mut current_song = self
                .song
                .lock()
                .map_err(|e| BackendError::MutexPoisoned(e.to_string()))?;
            *current_song = Some(song.clone());
        }

        // Play the song
        self.player.play(&url).map_err(BackendError::Mpv)?;

        // Add the song to history
        self.history
            .add_entry(&HistoryEntry::from(song))
            .map_err(|e| BackendError::HistoryError(e.to_string()))?;

        Ok(())
    }
}
