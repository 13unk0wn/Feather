// This file manages the history database and contains all necessary functions related to history management
use crate::{ArtistName, SongId, SongName};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Represents a history entry for a song that has been played.
#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryEntry {
    pub song_name: SongName,          // Name of the song
    pub song_id: SongId,              // Unique identifier for the song
    pub artist_name: Vec<ArtistName>, // List of artists associated with the song
    time_stamp: u64,                  // Timestamp when the song was played
}

impl HistoryEntry {
    /// Creates a new history entry with the current timestamp.
    pub fn new(
        song_name: SongName,
        song_id: SongId,
        artist_name: Vec<ArtistName>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let time_stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        Ok(Self {
            song_name,
            song_id,
            artist_name,
            time_stamp,
        })
    }
}

/// Database handler for managing song history.
pub struct HistoryDB {
    db: Db, // Sled database instance
}

/// Represents possible errors that can occur in history operations.
#[derive(Error, Debug)]
pub enum HistoryError {
    #[error("Database error: {0}")]
    DbError(#[from] sled::Error), // Errors related to the sled database
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error), // Errors during serialization/deserialization
    #[error("Basic error: {0}")]
    Error(Box<dyn std::error::Error>), // Generic error wrapper
}

impl HistoryDB {
    pub fn new() -> Result<Self, sled::Error> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push("Feather/history_db");

        let db = sled::Config::new()
            .path(path)
            .cache_capacity(256 * 1024)
            .use_compression(true)
            .open()?;

        Ok(HistoryDB { db })
    }

    /// Adds a new entry to the history database.
    /// Limits the total stored entries to 50.
    pub fn add_entry(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        let key = entry.song_id.as_bytes();
        let value = bincode::serialize(entry)?;
        self.db.insert(key, value)?;
        self.limit_history_size(50)?;
        Ok(())
    }

    /// Ensures the history database does not exceed `max_size` entries.
    /// Removes the oldest entries if necessary.
    pub fn limit_history_size(&self, max_size: usize) -> Result<(), HistoryError> {
        while self.db.len() > max_size {
            if let Some((key, _)) = self.db.first()? {
                self.db.remove(key)?;
            }
        }
        Ok(())
    }

    /// Retrieves up to 50 history entries, sorted by most recent first.
    pub fn get_history(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        let mut history = Vec::with_capacity(self.db.len().min(50)); // Pre-allocate vector
        for item in self.db.iter().take(50) {
            let (_, value) = item?;
            if let Ok(entry) = bincode::deserialize::<HistoryEntry>(&value) {
                history.push(entry);
            }
        }
        history.sort_unstable_by(|e1, e2| e2.time_stamp.cmp(&e1.time_stamp)); // Sort by timestamp descending
        Ok(history)
    }

    /// Deletes a specific history entry by song ID.
    pub fn delete_entry(&self, song_id: &str) -> Result<(), HistoryError> {
        self.db.remove(song_id.as_bytes())?; // Convert song ID to bytes
        Ok(())
    }

    /// Clears all history entries from the database.
    pub fn clear_history(&self) -> Result<(), HistoryError> {
        self.db.clear()?;
        Ok(())
    }

    /// Retrieves the most recently played song's ID, if available.
    pub fn get_last_played_song(&self) -> Result<Option<SongId>, HistoryError> {
        if let Some((_, last_entry)) = self.db.last()? {
            let entry: HistoryEntry = bincode::deserialize(&last_entry)?;
            Ok(Some(entry.song_id))
        } else {
            Ok(None)
        }
    }
}

// Unchanged UserPlaylist and PlaylistManager sections...
// #[derive(Serialize, Deserialize, Debug, Clone)]
// struct UserPlaylist {
//     playlist_name: PlaylistName,
//     songs: Vec<Song>,
// }

// #[derive(Error, Debug)]
// pub enum PlaylistManagerError {
//     #[error("Database error: {0}")]
//     DbError(#[from] sled::Error),
//     #[error("Serialization error: {0}")]
//     SerializationError(#[from] bincode::Error),
//     #[error("Playlist '{0}' not found")]
//     PlaylistNotFound(String),
//     #[error("Song '{0}' not found in playlist '{1}'")]
//     SongNotFound(String, String),
//     #[error("Duplicate playlist name: '{0}'")]
//     DuplicatePlaylist(String),
//     #[error("Failed to add song '{0}' to playlist '{1}'")]
//     AddSongError(String, String),
//     #[error("Failed to remove song '{0}' from playlist '{1}'")]
//     RemoveSongError(String, String),
//     #[error("Unknown error: {0}")]
//     Other(String),
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// struct Song {
//     song_name: SongName,
//     song_id: SongId,
//     artist: Vec<ArtistName>,
// }

// struct PlaylistManager {
//     db: sled::Db,
// }

// impl PlaylistManager {
//     pub fn new(path: &str) -> Result<Self, PlaylistManagerError> {
//         let db = sled::open(path)?;
//         Ok(Self { db })
//     }
//     fn create_playlist(&self, name: &str) -> Result<(), PlaylistManagerError> {
//         if self.db.get(name)?.is_some() {
//             return Err(PlaylistManagerError::DuplicatePlaylist(name.to_string()));
//         }
//         let playlist = UserPlaylist {
//             playlist_name: name.to_string(),
//             songs: Vec::new(),
//         };
//         let value = bincode::serialize(&playlist)?;
//         self.db.insert(name, value)?;
//         self.db.flush()?;
//         Ok(())
//     }
//     fn add_song_to_playlist(
//         &self,
//         playlist_name: &str,
//         song: Song,
//     ) -> Result<(), PlaylistManagerError> {
//         let raw_data = self
//             .db
//             .get(playlist_name)?
//             .ok_or_else(|| PlaylistManagerError::Other("Error: In Opening Playlist".to_string()))?
//             .to_vec();

//         let mut playlist: UserPlaylist = bincode::deserialize(&raw_data)?;

//         playlist.songs.retain(|s| s.song_id != song.song_id);
//         playlist.songs.push(song);

//         let serialized_data = bincode::serialize(&playlist)?;
//         self.db.insert(playlist_name, serialized_data)?;
//         self.db.flush()?;

//         Ok(())
//     }
//     fn remove_song_from_playlist(
//         &self,
//         playlist_name: &str,
//         song_id: &str,
//     ) -> Result<(), PlaylistManagerError> {
//         let raw_data = self
//             .db
//             .get(playlist_name)?
//             .ok_or_else(|| PlaylistManagerError::Other("Error: In Opening Playlist".to_string()))?
//             .to_vec();

//         let mut playlist: UserPlaylist = bincode::deserialize(&raw_data)?;

//         playlist.songs.retain(|s| s.song_id != song_id);
//         let serialized_data = bincode::serialize(&playlist)?;
//         self.db.insert(playlist_name, serialized_data)?;
//         self.db.flush()?;

//         Ok(())
//     }

//     fn get_playlist(&self, playlist_name: &str) -> Result<UserPlaylist, PlaylistManagerError> {
//         let data = self
//             .db
//             .get(playlist_name)?
//             .ok_or_else(|| PlaylistManagerError::PlaylistNotFound(playlist_name.to_string()))?
//             .to_vec();
//         let playlist: UserPlaylist = bincode::deserialize(&data)?;
//         Ok(playlist)
//     }
//     fn delete_playlist(&self, playlist_name: &str) -> Result<(), PlaylistManagerError> {
//         self.db
//             .remove(&playlist_name)?
//             .ok_or_else(|| PlaylistManagerError::PlaylistNotFound(playlist_name.to_string()));
//         self.db.flush()?;
//         Ok(())
//     }
// }

// // Tests unchanged...
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tempfile::tempdir;

//     fn sample_song(name: &str, id: &str) -> Song {
//         Song {
//             song_name: name.to_string(),
//             song_id: id.to_string(),
//             artist: vec!["Artist One".to_string(), "Artist Two".to_string()],
//         }
//     }

//     #[test]
//     fn test_playlist_manager() {
//         let temp_dir = tempdir().unwrap();
//         let db_path = temp_dir.path().to_str().unwrap();
//         let manager = PlaylistManager::new(db_path).unwrap();

//         let playlist_name = "MyPlaylist";

//         assert!(manager.create_playlist(playlist_name).is_ok());

//         let song1 = sample_song("Song A", "123");
//         let song2 = sample_song("Song B", "456");

//         assert!(manager
//             .add_song_to_playlist(playlist_name, song1.clone())
//             .is_ok());
//         assert!(manager
//             .add_song_to_playlist(playlist_name, song2.clone())
//             .is_ok());

//         let playlist = manager.get_playlist(playlist_name).unwrap();
//         assert_eq!(playlist.songs.len(), 2);
//         assert!(playlist.songs.iter().any(|s| s.song_id == "123"));
//         assert!(playlist.songs.iter().any(|s| s.song_id == "456"));

//         assert!(manager
//             .remove_song_from_playlist(playlist_name, "123")
//             .is_ok());
//         let playlist = manager.get_playlist(playlist_name).unwrap();
//         assert_eq!(playlist.songs.len(), 1);
//         assert!(playlist.songs.iter().all(|s| s.song_id != "123"));

//         assert!(manager.delete_playlist(playlist_name).is_ok());
//         let result = manager.get_playlist(playlist_name);
//         assert!(matches!(
//             result,
//             Err(PlaylistManagerError::PlaylistNotFound(_))
//         ));
//     }
// }
