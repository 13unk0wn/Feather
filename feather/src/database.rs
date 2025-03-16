#![allow(unused)]
use crate::yt::YoutubeClient;
use crate::PlaylistName;
use log::debug;
use log::log;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTimeError;
// This file manages the history database and contains all necessary functions related to history management
use crate::{ArtistName, SongId, SongName};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sled::Db;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MIGRATION_KEY: &str = "DONE";

/// Represents a history entry for a song that has been played.
#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryEntry {
    pub song_name: SongName,          // Name of the song
    pub song_id: SongId,              // Unique identifier for the song
    pub artist_name: Vec<ArtistName>, // List of artists associated with the song
    time_stamp: u64,                  // Timestamp when the song was played
    pub play_count: u64,
}

impl HistoryEntry {
    /// Creates a new history entry with the current timestamp.
    pub fn new(
        song_name: SongName,
        song_id: SongId,
        artist_name: Vec<ArtistName>,
    ) -> Result<Self, HistoryError> {
        let time_stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        Ok(Self {
            song_name,
            song_id,
            artist_name,
            time_stamp,
            play_count: 1,
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
    #[error("Time Erorr : {0}")]
    Erorr(#[from] SystemTimeError),
}

impl HistoryDB {
    pub fn new() -> Result<Self, HistoryError> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push("Feather/history_db");

        let db = sled::Config::new()
            .path(path)
            .cache_capacity(256 * 1024)
            .use_compression(true)
            .open()?;

        let db = HistoryDB { db };
        db.migrate_history()?;
        Ok(db)
    }
    pub fn backup_history(&self) -> Result<(), HistoryError> {
        let backup_path = Path::new("history_backup.bin");
        let mut backup_file = File::create(backup_path).unwrap();

        // Collect all history entries
        let mut history_entries = Vec::new();
        for item in self.db.iter() {
            let (_, value) = item?;
            if let Ok(entry) = bincode::deserialize::<HistoryEntry>(&value) {
                history_entries.push(entry);
            }
        }

        // Serialize and write to the backup file
        bincode::serialize_into(&mut backup_file, &history_entries)?;

        Ok(())
    }

    pub fn migrate_history(&self) -> Result<(), HistoryError> {
        // backup history
        if self.db.get(MIGRATION_KEY)?.is_some() {
            return Ok(());
        }
        self.backup_history()?;
        for item in self.db.iter() {
            let (key, value) = item?;
            if let Ok(mut entry) = bincode::deserialize::<HistoryEntry>(&value) {
                if entry.play_count == 0 {
                    entry.play_count = 1; // Default to 1 if missing
                    let new_value = bincode::serialize(&entry)?;
                    self.db.insert(key, new_value)?; // Update database
                }
            }
        }
        self.db.insert(MIGRATION_KEY, b"true")?;
        Ok(())
    }

    /// Adds a new entry to the history database.
    /// Limits the total stored entries to 50.
    pub fn add_entry(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        let key = entry.song_id.as_bytes();

        if let Some(value) = self.db.get(key)? {
            let mut existing_entry: HistoryEntry = bincode::deserialize(&value)?;
            existing_entry.play_count += 1; // Increase play count
            existing_entry.time_stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(); // Update timestamp

            let new_value = bincode::serialize(&existing_entry)?;
            self.db.insert(key, new_value)?;
        } else {
            // If it's a new song, add it normally
            let new_value = bincode::serialize(entry)?;
            self.db.insert(key, new_value)?;
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

use std::str;

pub const PAGE_SIZE: usize = 20;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub artist_name: Vec<String>,
}

/// Implements conversion from `Song` to `HistoryEntry`, ensuring valid history records.
impl From<Song> for HistoryEntry {
    fn from(value: Song) -> Self {
        HistoryEntry::new(value.title, value.id, value.artist_name)
            .expect("Cannot Form History Entry")
    }
}

impl Song {
    pub fn new(id: String, title: String, artist_name: Vec<String>) -> Self {
        Self {
            id,
            title,
            artist_name,
        }
    }
}
#[derive(Error, Debug)]
pub enum SongError {
    #[error("Database error: {0}")]
    DbError(#[from] sled::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Invalid UTF-8 sequence")]
    Utf8Error,

    #[error("File exit already")]
    FileExist(#[from] std::io::Error),

    #[error("Song Not Found")]
    SongNotFound,
}

#[derive(Clone)]
pub struct SongDatabase {
    pub db: Db,
    db_path: PathBuf,
    current_index: usize,
}

impl Drop for SongDatabase {
    fn drop(&mut self) {
        self.db.flush();
    }
}

impl SongDatabase {
    pub fn new(name: &str) -> Result<Self, SongError> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push(format!("Feather/{}", name));

        // Check if the path exists, and delete accordingly
        if path.exists() {
            if path.is_dir() {
                fs::remove_dir_all(&path).map_err(|e| SongError::FileExist(e))?;
            } else {
                fs::remove_file(&path).map_err(|e| SongError::FileExist(e))?;
            }
        }

        let path_clone = path.clone();
        let db = sled::open(path)?;

        Ok(Self {
            db,
            db_path: path_clone,
            current_index: 0,
        })
    }

    pub fn add_song(
        &mut self,
        title: String,
        id: String,
        artist_name: Vec<String>,
    ) -> Result<(), SongError> {
        let song = Song {
            id,
            title,
            artist_name,
        };
        let key = format!("song:{}", self.current_index);
        let value = serde_json::to_vec(&song)?;
        self.db.insert(key, value)?;
        self.current_index += 1;
        Ok(())
    }

    pub fn get_song_by_index(&self, index: usize) -> Result<Song, SongError> {
        let key = format!("song:{}", index); // Format the key as you did in `add_song`
        if let Some(value) = self.db.get(key)? {
            let song: Song = serde_json::from_slice(&value)?;
            Ok(song)
        } else {
            Err(SongError::SongNotFound)
        }
    }
    //TODO :  Change the logic it  is not working
    pub fn next_page(&self, offset: usize) -> Result<Vec<Song>, SongError> {
        let mut songs: Vec<Song> = self
            .db
            .iter()
            .filter_map(|res| match res {
                Ok((key, v)) => {
                    // Convert the Vec<u8> key to a string
                    let key_str = String::from_utf8_lossy(&key).to_string();

                    // Derive current_index from the key (e.g., "song:123")
                    let current_index: usize = key_str
                        .strip_prefix("song:")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_default();

                    // Deserialize the song data from the value
                    serde_json::from_slice(&v)
                        .map(|song: Song| (current_index, song))
                        .ok()
                }
                Err(_) => None,
            })
            .filter(|&(current_index, _)| {
                current_index >= offset && current_index < offset + PAGE_SIZE
            })
            .map(|(_, song)| song)
            .collect();

        // Sort the songs based on the extracted `current_index`
        songs.sort_by_key(|s| {
            let key = format!(
                "song:{}",
                self.db.iter().position(|item| item.is_ok()).unwrap_or(0)
            );
            key
        });

        Ok(songs)
    }
}

// Unchanged UserPlaylist and PlaylistManager sections...
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserPlaylist {
    playlist_name: PlaylistName,
    max_index: usize,
    songs: Vec<(usize, Song)>,
}

impl UserPlaylist {
    fn new(playlist_name: PlaylistName) -> Self {
        Self {
            playlist_name,
            max_index: 0,
            songs: Vec::new(),
        }
    }
}

#[derive(Error, Debug)]
pub enum PlaylistManagerError {
    #[error("Database error: {0}")]
    DbError(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
    #[error("Playlist '{0}' not found")]
    PlaylistNotFound(String),
    #[error("Song '{0}' not found in playlist '{1}'")]
    SongNotFound(String, String),
    #[error("Duplicate playlist name: '{0}'")]
    DuplicatePlaylist(String),
    #[error("Failed to add song '{0}' to playlist '{1}'")]
    AddSongError(String, String),
    #[error("Failed to remove song '{0}' from playlist '{1}'")]
    RemoveSongError(String, String),
    #[error("Conversion Error  : {0}")]
    SongError(#[from] SongError),
    #[error("Unknown error: {0}")]
    Other(String),
}

pub struct PlaylistManager {
    db: sled::Db,
}

impl PlaylistManager {
    pub fn new() -> Result<Self, PlaylistManagerError> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push("Feather/UserPlaylist_db");
        let db = sled::open(path)?;
        Ok(Self { db })
    }
    pub fn create_playlist(&self, name: &str) -> Result<(), PlaylistManagerError> {
        if self.db.get(name)?.is_some() {
            return Err(PlaylistManagerError::DuplicatePlaylist(name.to_string()));
        }
        let playlist = UserPlaylist::new(name.to_string());
        let value = bincode::serialize(&playlist)?;
        self.db.insert(name, value)?;
        self.db.flush()?;
        Ok(())
    }
    pub fn convert_playlist(
        &self,
        playlist_name: &str,
    ) -> Result<SongDatabase, PlaylistManagerError> {
        let mut song_playlist = SongDatabase::new(playlist_name)?;

        let get_playlist = self.get_playlist(playlist_name)?;

        for i in get_playlist {
            song_playlist.add_song(i.title, i.id, i.artist_name);
        }
        Ok(song_playlist)
    }
    pub fn add_song_to_playlist(
        &self,
        playlist_name: &str,
        song: Song,
    ) -> Result<(), PlaylistManagerError> {
        let raw_data = self
            .db
            .get(playlist_name)?
            .ok_or_else(|| PlaylistManagerError::Other("Error: In Opening Playlist".to_string()))?
            .to_vec();

        let mut playlist: UserPlaylist = bincode::deserialize(&raw_data)?;

        playlist.songs.retain(|s| s.1.id != song.id);
        playlist.songs.push((playlist.max_index, song));

        playlist.max_index += 1;
        let serialized_data = bincode::serialize(&playlist)?;
        self.db.insert(playlist_name, serialized_data)?;
        self.db.flush()?;

        Ok(())
    }
    pub fn list_playlists(&self) -> Result<Vec<String>, PlaylistManagerError> {
        self.db
            .iter()
            .keys()
            .map(|key| {
                key.map(|k| String::from_utf8_lossy(&k).into_owned())
                    .map_err(PlaylistManagerError::from)
            })
            .collect()
    }
    pub fn remove_song_from_playlist(
        &self,
        playlist_name: &str,
        song_id: &str,
    ) -> Result<(), PlaylistManagerError> {
        let raw_data = self
            .db
            .get(playlist_name)?
            .ok_or_else(|| PlaylistManagerError::Other("Error: In Opening Playlist".to_string()))?
            .to_vec();

        let mut playlist: UserPlaylist = bincode::deserialize(&raw_data)?;

        playlist.songs.retain(|s| s.1.id != song_id);
        let serialized_data = bincode::serialize(&playlist)?;
        self.db.insert(playlist_name, serialized_data)?;
        self.db.flush()?;

        Ok(())
    }

    pub fn get_playlist(&self, playlist_name: &str) -> Result<Vec<Song>, PlaylistManagerError> {
        let data = self
            .db
            .get(playlist_name)?
            .ok_or_else(|| PlaylistManagerError::PlaylistNotFound(playlist_name.to_string()))?
            .to_vec();

        let mut playlist: UserPlaylist = bincode::deserialize(&data)?;

        playlist.songs.sort_by_key(|song| song.0);

        // Use PAGE_SIZE for pagination
        let songs = playlist.songs.into_iter().map(|s| s.1).collect::<Vec<_>>();
        Ok(songs)
    }
    pub fn get_user_playlist(
        &self,
        playlist_name: &str,
    ) -> Result<UserPlaylist, PlaylistManagerError> {
        let raw_data = self
            .db
            .get(playlist_name)?
            .ok_or_else(|| PlaylistManagerError::PlaylistNotFound(playlist_name.to_string()))?
            .to_vec();

        let user_playlist: UserPlaylist = bincode::deserialize(&raw_data)?;

        Ok(user_playlist) // Now explicitly returning a `UserPlaylist`
    }

    pub fn delete_playlist(&self, playlist_name: &str) -> Result<(), PlaylistManagerError> {
        self.db
            .remove(&playlist_name)?
            .ok_or_else(|| PlaylistManagerError::PlaylistNotFound(playlist_name.to_string()));
        self.db.flush()?;
        Ok(())
    }
}

// #[derive(Serialize, Deserialize, Debug)]
// struct UserProfile {
//     name: String,
//     ascii_image: String,
//     uptime: u64,
//     last_played: String,
//     total_songs_played: u64,
// }
// impl UserProfile {
//     fn new(name: String, ascii_image: String, uptime: u64) -> Self {
//         UserProfile { name, ascii_image,last_played  :  };
//         unimplemented!()
//     }
// }

struct UserProfileDb {
    db: sled::Db,
}

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
