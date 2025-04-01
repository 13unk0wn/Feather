#![allow(unused, non_camel_case_types)]
use crate::PlaylistName;
use crate::config::USERCONFIG;
use crate::yt::YoutubeClient;
use bincode::Deserializer;
use bincode::config;
use log::debug;
use log::log;
use rascii_art::RenderOptions;
use rascii_art::render_to;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::SystemTimeError;
// This file manages the history database and contains all necessary functions related to history management
use crate::{ArtistName, SongId, SongName};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sled::Db;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use sys_info::hostname;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct oldHistoryEntry {
    pub song_name: SongName,          // Name of the song
    pub song_id: SongId,              // Unique identifier for the song
    pub artist_name: Vec<ArtistName>, // List of artists associated with the song
    time_stamp: u64,
}

impl oldHistoryEntry {
    fn convert(self) -> HistoryEntry {
        HistoryEntry {
            song_name: self.song_name,
            song_id: self.song_id,
            artist_name: self.artist_name,
            time_stamp: self.time_stamp,
            play_count: 1,
        }
    }
}
pub const FAVOURITE_SONGS_SIZE: usize = 5;
pub const HISTORY_PAGE_SIZE: usize = 20;
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
    pub db: Db, // Sled database instance
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
            if let Ok(entry) = bincode::deserialize::<oldHistoryEntry>(&value) {
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
            if let Ok(mut entry) = bincode::deserialize::<oldHistoryEntry>(&value) {
                let new_entry = entry.convert();
                let new_entry = bincode::serialize(&new_entry)?;
                self.db.insert(key, new_entry)?;
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
    pub fn get_history(&self, offset: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let mut history = Vec::new();
        for item in self.db.iter() {
            let (_, value) = item?;
            if let Ok(entry) = bincode::deserialize::<HistoryEntry>(&value) {
                history.push(entry);
            }
        }

        // Sort by timestamp in descending order
        history.sort_unstable_by(|e1, e2| e2.time_stamp.cmp(&e1.time_stamp));

        // Apply offset and take the required number of entries
        Ok(history
            .into_iter()
            .skip(offset)
            .take(HISTORY_PAGE_SIZE)
            .collect())
    }
    /// most played  5 songs.
    pub fn most_played(&self) -> Result<Vec<HistoryEntry>, HistoryError> {
        let mut history = Vec::new();
        for item in self.db.iter() {
            let (_, value) = item?;
            if let Ok(entry) = bincode::deserialize::<HistoryEntry>(&value) {
                history.push(entry);
            }
        }

        // Sort by timestamp in descending order
        history.sort_unstable_by(|e1, e2| e2.play_count.cmp(&e1.play_count));

        // Apply offset and take the required number of entries
        Ok(history.into_iter().take(FAVOURITE_SONGS_SIZE).collect())
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
        let key = format!("{}", self.current_index);
        let value = serde_json::to_vec(&song)?;
        self.db.insert(key, value)?;
        self.current_index += 1;
        Ok(())
    }

    pub fn get_song_by_index(&self, index: usize) -> Result<Song, SongError> {
        let key = format!("{}", index); // Format the key as you did in `add_song`
        if let Some(value) = self.db.get(key)? {
            let song: Song = serde_json::from_slice(&value)?;
            Ok(song)
        } else {
            Err(SongError::SongNotFound)
        }
    }
    //TODO :  Change the logic it  is not working
    pub fn next_page(&self, offset: usize) -> Result<Vec<Song>, SongError> {
        let mut songs_with_index: Vec<(usize, Song)> = self
            .db
            .iter()
            .filter_map(|res| match res {
                Ok((key, value)) => {
                    // Convert key from Vec<u8> to string and parse it as usize
                    let key_str = String::from_utf8_lossy(&key);
                    let index: usize = key_str.parse().unwrap_or(usize::MAX); // Use MAX as fallback for invalid keys

                    // Deserialize the song
                    serde_json::from_slice(&value)
                        .map(|song: Song| (index, song))
                        .ok()
                }
                Err(_) => None,
            })
            .collect();

        // Sort by index to ensure correct order
        songs_with_index.sort_by_key(|&(index, _)| index);

        // Filter and take the songs for the current page
        let songs: Vec<Song> = songs_with_index
            .into_iter()
            .filter(|&(index, _)| index >= offset && index < offset + PAGE_SIZE)
            .map(|(_, song)| song)
            .collect();

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
        let mut song_playlist = SongDatabase::new("load_playlist")?;

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

const DEFAULT_PFP: &str = "\u{1b}[38;2;1;1;1m  \u{1b}[38;2;2;2;2m \u{1b}[38;2;2;2;3m \u{1b}[38;2;2;3;4m \u{1b}[38;2;4;4;6m.\u{1b}[38;2;5;6;7m.\u{1b}[38;2;9;10;11m`\u{1b}[38;2;14;15;16m\"\u{1b}[38;2;19;20;20m\\\u{1b}[38;2;25;26;25m,\u{1b}[38;2;31;31;31m;;\u{1b}[38;2;35;36;37mI\u{1b}[38;2;40;41;41m!\u{1b}[38;2;49;49;52m>\u{1b}[38;2;63;64;65m_\u{1b}[38;2;61;64;64m_\u{1b}[38;2;58;59;58m~\u{1b}[38;2;53;54;53m<\u{1b}[38;2;48;49;48m>\u{1b}[38;2;41;44;43m!\u{1b}[38;2;38;39;39ml\u{1b}[38;2;35;36;36mI\u{1b}[38;2;33;34;34mI\u{1b}[38;2;33;33;33m;\u{1b}[38;2;32;32;32m;\u{1b}[38;2;31;33;32m;\u{1b}[38;2;32;33;33m;\u{1b}[38;2;33;34;33mI\u{1b}[38;2;37;37;36mI\u{1b}[38;2;41;41;39m!\u{1b}[38;2;43;44;39m!\u{1b}[38;2;47;45;43mi\u{1b}[38;2;47;47;43mi\u{1b}[38;2;46;45;41mi\u{1b}[38;2;44;42;40m!\u{1b}[38;2;42;40;37ml\u{1b}[38;2;38;37;35ml\u{1b}[38;2;34;34;32mI\u{1b}[38;2;31;31;30m;\u{1b}[38;2;27;28;28m:\u{1b}[38;2;28;28;28m:\u{1b}[38;2;29;29;29m:\u{1b}[38;2;32;32;33m;\u{1b}[38;2;37;37;37ml\u{1b}[38;2;44;45;44mi\u{1b}[38;2;50;52;51m>\u{1b}[38;2;51;53;51m<\u{1b}[38;2;53;55;53m<\u{1b}[38;2;55;56;56m~\u{1b}[38;2;56;58;57m~\u{1b}[38;2;58;59;60m~\u{1b}[38;2;59;61;60m+\u{1b}[38;2;60;62;61m++\u{1b}[38;2;61;63;62m+\u{1b}[38;2;62;64;63m_\u{1b}[38;2;63;65;63m_\u{1b}[38;2;64;66;66m_\u{1b}[0m\n\u{1b}[38;2;2;2;2m  \u{1b}[38;2;2;3;2m \u{1b}[38;2;2;4;4m \u{1b}[38;2;4;6;5m.\u{1b}[38;2;5;7;7m.\u{1b}[38;2;8;10;9m`\u{1b}[38;2;12;13;14m^\u{1b}[38;2;16;18;17m\"\u{1b}[38;2;20;22;21m\\\u{1b}[38;2;22;24;23m,\u{1b}[38;2;26;27;28m:\u{1b}[38;2;31;30;31m;\u{1b}[38;2;24;26;25m,\u{1b}[38;2;26;26;26m:\u{1b}[38;2;27;27;26m:\u{1b}[38;2;28;28;28m:\u{1b}[38;2;37;38;38ml\u{1b}[38;2;48;48;47mi\u{1b}[38;2;46;48;47mi\u{1b}[38;2;43;45;42m!\u{1b}[38;2;41;42;41m!\u{1b}[38;2;37;39;38ml\u{1b}[38;2;35;35;35mI\u{1b}[38;2;34;34;34mIII\u{1b}[38;2;35;35;35mI\u{1b}[38;2;36;36;36mI\u{1b}[38;2;37;37;33mI\u{1b}[38;2;37;37;35mI\u{1b}[38;2;28;30;28m:\u{1b}[38;2;21;21;19m\\\u{1b}[38;2;14;13;12m^\u{1b}[38;2;7;7;7m...\u{1b}[38;2;6;7;6m.\u{1b}[38;2;5;7;6m.\u{1b}[38;2;5;6;6m..\u{1b}[38;2;5;7;6m.\u{1b}[38;2;5;6;6m.\u{1b}[38;2;5;6;7m.\u{1b}[38;2;6;6;8m.\u{1b}[38;2;5;5;7m.\u{1b}[38;2;5;6;8m.\u{1b}[38;2;9;10;11m`\u{1b}[38;2;27;28;28m:\u{1b}[38;2;54;55;55m<\u{1b}[38;2;59;60;58m+\u{1b}[38;2;61;62;61m+\u{1b}[38;2;63;65;64m_\u{1b}[38;2;64;66;65m_\u{1b}[38;2;65;67;66m_\u{1b}[38;2;66;68;67m-\u{1b}[38;2;67;69;68m-\u{1b}[38;2;68;70;69m--\u{1b}[38;2;67;70;70m-\u{1b}[0m\n\u{1b}[38;2;1;3;3m \u{1b}[38;2;2;3;4m \u{1b}[38;2;3;5;5m.\u{1b}[38;2;4;6;5m.\u{1b}[38;2;7;9;8m`\u{1b}[38;2;9;11;11m`\u{1b}[38;2;11;13;13m^\u{1b}[38;2;14;16;15m\"\u{1b}[38;2;17;19;18m\"\u{1b}[38;2;20;21;20m\\\u{1b}[38;2;22;23;22m,\u{1b}[38;2;23;25;24m,\u{1b}[38;2;26;27;29m:\u{1b}[38;2;27;27;28m:\u{1b}[38;2;15;16;16m\"\u{1b}[38;2;14;14;14m^\u{1b}[38;2;12;14;14m^\u{1b}[38;2;11;14;13m^\u{1b}[38;2;16;17;17m\"\u{1b}[38;2;30;29;30m:\u{1b}[38;2;37;39;38ml\u{1b}[38;2;39;39;39ml\u{1b}[38;2;38;38;38ml\u{1b}[38;2;36;36;36mI\u{1b}[38;2;35;36;36mII\u{1b}[38;2;36;36;36mI\u{1b}[38;2;36;37;37mI\u{1b}[38;2;30;32;32m;\u{1b}[38;2;17;16;18m\"\u{1b}[38;2;6;6;6m.\u{1b}[38;2;5;5;5m.\u{1b}[38;2;6;6;6m.\u{1b}[38;2;6;7;6m.\u{1b}[38;2;8;7;7m.\u{1b}[38;2;34;36;35mI\u{1b}[38;2;80;87;87m}\u{1b}[38;2;87;92;93m1\u{1b}[38;2;102;107;108m\\\u{1b}[38;2;118;125;125mr\u{1b}[38;2;123;129;129mx\u{1b}[38;2;123;128;127mx\u{1b}[38;2;161;163;162mJ\u{1b}[38;2;169;170;170mL\u{1b}[38;2;148;149;151mX\u{1b}[38;2;103;105;108m\\\u{1b}[38;2;24;26;30m,\u{1b}[38;2;4;5;6m.\u{1b}[38;2;5;6;7m.\u{1b}[38;2;13;14;14m^\u{1b}[38;2;56;57;56m~\u{1b}[38;2;67;69;68m-\u{1b}[38;2;69;71;70m?\u{1b}[38;2;71;73;73m?\u{1b}[38;2;72;74;75m?\u{1b}[38;2;73;75;76m]\u{1b}[38;2;73;75;77m]\u{1b}[38;2;73;77;78m]\u{1b}[38;2;75;76;78m]\u{1b}[38;2;73;77;78m]\u{1b}[0m\n\u{1b}[38;2;3;4;4m.\u{1b}[38;2;3;4;5m.\u{1b}[38;2;4;6;6m.\u{1b}[38;2;6;8;7m.\u{1b}[38;2;8;10;9m`\u{1b}[38;2;12;14;14m^\u{1b}[38;2;14;16;16m\"\u{1b}[38;2;15;17;16m\"\u{1b}[38;2;17;18;19m\"\u{1b}[38;2;18;19;20m\\\u{1b}[38;2;20;21;22m\\\u{1b}[38;2;22;24;23m,\u{1b}[38;2;24;26;26m,\u{1b}[38;2;25;27;27m:\u{1b}[38;2;24;25;25m,\u{1b}[38;2;10;11;11m`\u{1b}[38;2;6;9;9m`\u{1b}[38;2;9;10;10m`\u{1b}[38;2;48;50;49m>\u{1b}[38;2;18;20;21m\\\u{1b}[38;2;14;14;16m^\u{1b}[38;2;25;26;27m,\u{1b}[38;2;35;36;35mI\u{1b}[38;2;38;38;38mlll\u{1b}[38;2;37;37;37ml\u{1b}[38;2;17;20;20m\\\u{1b}[38;2;4;5;5m.\u{1b}[38;2;4;4;5m.\u{1b}[38;2;31;30;31m;\u{1b}[38;2;72;73;72m?\u{1b}[38;2;107;109;107m/\u{1b}[38;2;136;139;137mv\u{1b}[38;2;155;157;155mU\u{1b}[38;2;160;162;161mJ\u{1b}[38;2;162;164;162mC\u{1b}[38;2;161;164;161mC\u{1b}[38;2;160;165;160mC\u{1b}[38;2;160;166;162mC\u{1b}[38;2;161;167;161mC\u{1b}[38;2;164;169;165mL\u{1b}[38;2;177;183;177mO\u{1b}[38;2;248;249;248mB\u{1b}[38;2;255;255;255m@\u{1b}[38;2;255;255;254m$\u{1b}[38;2;234;236;237mW\u{1b}[38;2;51;51;54m>\u{1b}[38;2;5;6;7m.\u{1b}[38;2;7;8;9m`\u{1b}[38;2;18;19;19m\\\u{1b}[38;2;79;81;80m[\u{1b}[38;2;83;84;84m}\u{1b}[38;2;83;85;85m}\u{1b}[38;2;85;86;86m{\u{1b}[38;2;85;87;87m{\u{1b}[38;2;86;87;89m{\u{1b}[38;2;87;88;88m{\u{1b}[38;2;86;88;88m{\u{1b}[38;2;86;87;88m{\u{1b}[0m\n\u{1b}[38;2;5;7;6m.\u{1b}[38;2;6;8;7m.\u{1b}[38;2;7;9;9m`\u{1b}[38;2;9;11;10m`\u{1b}[38;2;12;14;13m^\u{1b}[38;2;14;16;15m\"\u{1b}[38;2;16;18;17m\"\u{1b}[38;2;18;20;19m\\\u{1b}[38;2;18;20;20m\\\u{1b}[38;2;19;21;20m\\\u{1b}[38;2;20;22;21m\\\u{1b}[38;2;22;24;23m,\u{1b}[38;2;24;26;25m,\u{1b}[38;2;26;27;27m:\u{1b}[38;2;28;28;28m:\u{1b}[38;2;23;24;24m,\u{1b}[38;2;7;9;8m`\u{1b}[38;2;5;6;6m.\u{1b}[38;2;30;31;30m;\u{1b}[38;2;137;139;136mv\u{1b}[38;2;95;98;96m(\u{1b}[38;2;47;50;49m>\u{1b}[38;2;11;13;13m^\u{1b}[38;2;19;20;20m\\\u{1b}[38;2;28;30;28m:\u{1b}[38;2;31;33;32m;\u{1b}[38;2;14;15;13m^\u{1b}[38;2;5;5;5m.\u{1b}[38;2;48;48;47mi\u{1b}[38;2;130;130;127mn\u{1b}[38;2;167;167;163mL\u{1b}[38;2;165;167;162mCCC\u{1b}[38;2;165;168;161mL\u{1b}[38;2;165;167;162mC\u{1b}[38;2;150;151;145mX\u{1b}[38;2;96;96;92m)\u{1b}[38;2;85;86;82m{\u{1b}[38;2;98;100;95m(\u{1b}[38;2;161;163;159mJ\u{1b}[38;2;169;171;169mQ\u{1b}[38;2;172;175;170mQ\u{1b}[38;2;235;235;234mW\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;244;245;244m%\u{1b}[38;2;49;50;51m>\u{1b}[38;2;7;9;8m`\u{1b}[38;2;8;10;10m`\u{1b}[38;2;68;69;69m-\u{1b}[38;2;97;97;97m(\u{1b}[38;2;98;99;99m(\u{1b}[38;2;99;100;101m|\u{1b}[38;2;100;101;101m|\u{1b}[38;2;99;100;100m(\u{1b}[38;2;98;101;102m|\u{1b}[38;2;98;101;100m|\u{1b}[38;2;97;101;99m(\u{1b}[0m\n\u{1b}[38;2;6;8;7m.\u{1b}[38;2;7;9;8m`\u{1b}[38;2;9;11;10m`\u{1b}[38;2;11;12;12m^\u{1b}[38;2;13;15;14m^\u{1b}[38;2;16;18;18m\"\u{1b}[38;2;18;20;20m\\\u{1b}[38;2;19;21;20m\\\u{1b}[38;2;20;22;21m\\\u{1b}[38;2;20;22;22m\\\u{1b}[38;2;21;23;23m,\u{1b}[38;2;23;24;24m,\u{1b}[38;2;26;27;27m:\u{1b}[38;2;29;30;30m;\u{1b}[38;2;32;32;32m;\u{1b}[38;2;33;33;33m;\u{1b}[38;2;26;27;27m:\u{1b}[38;2;9;10;9m`\u{1b}[38;2;6;7;6m.\u{1b}[38;2;38;40;39ml\u{1b}[38;2;133;134;132mu\u{1b}[38;2;149;151;148mX\u{1b}[38;2;137;139;137mv\u{1b}[38;2;91;95;94m)\u{1b}[38;2;56;58;58m~\u{1b}[38;2;28;30;30m:\u{1b}[38;2;24;25;25m,\u{1b}[38;2;122;123;118mr\u{1b}[38;2;166;167;161mC\u{1b}[38;2;166;167;163mC\u{1b}[38;2;167;168;164mL\u{1b}[38;2;167;169;164mL\u{1b}[38;2;166;169;163mL\u{1b}[38;2;167;169;164mL\u{1b}[38;2;169;169;165mL\u{1b}[38;2;133;134;126mn\u{1b}[38;2;22;23;19m,\u{1b}[38;2;3;5;6m.\u{1b}[38;2;2;6;4m.\u{1b}[38;2;19;21;20m\\\u{1b}[38;2;153;158;153mU\u{1b}[38;2;171;175;171mQ\u{1b}[38;2;173;176;171m0\u{1b}[38;2;237;237;236m&\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;255;255;254m$\u{1b}[38;2;86;87;88m{\u{1b}[38;2;7;9;7m`\u{1b}[38;2;8;10;7m`\u{1b}[38;2;37;39;36ml\u{1b}[38;2;109;110;108m/\u{1b}[38;2;109;111;109m/\u{1b}[38;2;111;113;110mt\u{1b}[38;2;111;113;109mt\u{1b}[38;2;110;112;109mt\u{1b}[38;2;109;112;109m/\u{1b}[38;2;108;113;109mt\u{1b}[38;2;107;113;109m/\u{1b}[0m\n\u{1b}[38;2;7;8;9m`\u{1b}[38;2;8;9;10m`\u{1b}[38;2;9;11;11m`\u{1b}[38;2;11;13;12m^\u{1b}[38;2;14;16;14m\"\u{1b}[38;2;17;19;19m\"\u{1b}[38;2;19;21;21m\\\u{1b}[38;2;20;22;21m\\\u{1b}[38;2;21;23;22m,\u{1b}[38;2;23;25;23m,\u{1b}[38;2;24;27;25m:\u{1b}[38;2;26;28;27m:\u{1b}[38;2;30;30;30m;\u{1b}[38;2;31;32;32m;\u{1b}[38;2;35;35;35mI\u{1b}[38;2;37;38;38ml\u{1b}[38;2;39;39;39ml\u{1b}[38;2;33;34;32mI\u{1b}[38;2;11;11;10m`\u{1b}[38;2;9;9;8m`\u{1b}[38;2;26;27;26m:\u{1b}[38;2;125;125;123mr\u{1b}[38;2;154;156;153mY\u{1b}[38;2;158;158;156mU\u{1b}[38;2;161;161;158mJ\u{1b}[38;2;163;163;160mC\u{1b}[38;2;166;165;160mC\u{1b}[38;2;167;166;161mC\u{1b}[38;2;167;166;162mC\u{1b}[38;2;169;168;165mL\u{1b}[38;2;169;170;165mL\u{1b}[38;2;168;169;164mL\u{1b}[38;2;167;169;164mLL\u{1b}[38;2;136;139;132mv\u{1b}[38;2;10;17;8m^\u{1b}[38;2;3;12;5m`\u{1b}[38;2;4;9;6m.\u{1b}[38;2;53;59;57m~\u{1b}[38;2;140;147;144mz\u{1b}[38;2;165;169;166mL\u{1b}[38;2;168;172;169mQ\u{1b}[38;2;178;181;177mO\u{1b}[38;2;250;250;250mB\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;251;251;250mB\u{1b}[38;2;85;85;83m}\u{1b}[38;2;8;8;5m`\u{1b}[38;2;8;10;5m`\u{1b}[38;2;9;11;7m`\u{1b}[38;2;116;118;114mf\u{1b}[38;2;122;124;121mr\u{1b}[38;2;122;125;122mr\u{1b}[38;2;123;126;124mr\u{1b}[38;2;123;127;125mr\u{1b}[38;2;122;126;123mr\u{1b}[38;2;119;124;120mr\u{1b}[38;2;118;123;119mj\u{1b}[0m\n\u{1b}[38;2;9;11;10m`\u{1b}[38;2;10;12;12m^\u{1b}[38;2;11;13;13m^\u{1b}[38;2;13;15;14m^\u{1b}[38;2;14;16;15m\"\u{1b}[38;2;17;19;19m\"\u{1b}[38;2;21;23;23m,\u{1b}[38;2;23;24;25m,\u{1b}[38;2;23;25;25m,\u{1b}[38;2;25;27;26m:\u{1b}[38;2;27;29;28m:\u{1b}[38;2;29;31;32m;\u{1b}[38;2;31;33;33m;\u{1b}[38;2;34;35;35mI\u{1b}[38;2;38;38;38ml\u{1b}[38;2;41;41;41m!\u{1b}[38;2;43;44;43m!\u{1b}[38;2;42;44;40m!\u{1b}[38;2;38;39;38ml\u{1b}[38;2;18;18;16m\"\u{1b}[38;2;11;9;8m`\u{1b}[38;2;18;18;17m\"\u{1b}[38;2;131;132;127mn\u{1b}[38;2;160;160;157mJ\u{1b}[38;2;163;162;160mJ\u{1b}[38;2;165;164;160mC\u{1b}[38;2;166;165;160mC\u{1b}[38;2;167;167;161mL\u{1b}[38;2;168;167;163mL\u{1b}[38;2;169;168;164mL\u{1b}[38;2;168;169;164mL\u{1b}[38;2;168;168;164mL\u{1b}[38;2;166;168;165mL\u{1b}[38;2;165;168;164mL\u{1b}[38;2;162;165;162mC\u{1b}[38;2;120;126;122mr\u{1b}[38;2;108;116;112mt\u{1b}[38;2;140;147;145mz\u{1b}[38;2;163;169;167mL\u{1b}[38;2;164;169;168mL\u{1b}[38;2;163;167;165mC\u{1b}[38;2;167;171;167mL\u{1b}[38;2;176;179;174mO\u{1b}[38;2;250;250;249mB\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;254;255;254m$\u{1b}[38;2;143;143;142mc\u{1b}[38;2;7;9;3m`\u{1b}[38;2;8;10;4m`\u{1b}[38;2;9;11;6m`\u{1b}[38;2;125;130;125mx\u{1b}[38;2;136;140;137mv\u{1b}[38;2;136;141;137mv\u{1b}[38;2;137;141;138mv\u{1b}[38;2;136;141;138mv\u{1b}[38;2;134;140;136mv\u{1b}[38;2;131;137;132mu\u{1b}[38;2;126;134;130mn\u{1b}[0m\n\u{1b}[38;2;11;13;14m^\u{1b}[38;2;12;14;15m^\u{1b}[38;2;14;15;16m\"\u{1b}[38;2;16;18;16m\"\u{1b}[38;2;17;19;19m\"\u{1b}[38;2;20;21;22m\\\u{1b}[38;2;23;25;24m,\u{1b}[38;2;26;27;27m:\u{1b}[38;2;28;29;29m:\u{1b}[38;2;31;31;31m;\u{1b}[38;2;33;33;33m;\u{1b}[38;2;34;36;36mI\u{1b}[38;2;37;38;38ml\u{1b}[38;2;39;39;39ml\u{1b}[38;2;41;42;42m!\u{1b}[38;2;45;45;45mi\u{1b}[38;2;47;47;47mi\u{1b}[38;2;48;48;47mi\u{1b}[38;2;49;49;47m>\u{1b}[38;2;50;52;49m>\u{1b}[38;2;36;36;34mI\u{1b}[38;2;13;11;10m^\u{1b}[38;2;28;28;25m:\u{1b}[38;2;135;134;129mu\u{1b}[38;2;165;164;160mC\u{1b}[38;2;166;165;160mC\u{1b}[38;2;167;166;161mC\u{1b}[38;2;168;167;161mL\u{1b}[38;2;168;167;163mL\u{1b}[38;2;169;167;164mL\u{1b}[38;2;168;167;163mL\u{1b}[38;2;167;167;163mL\u{1b}[38;2;164;167;164mC\u{1b}[38;2;165;167;164mC\u{1b}[38;2;163;166;163mC\u{1b}[38;2;162;167;163mC\u{1b}[38;2;161;168;166mC\u{1b}[38;2;161;167;166mC\u{1b}[38;2;163;168;168mL\u{1b}[38;2;166;169;166mL\u{1b}[38;2;167;169;166mL\u{1b}[38;2;169;171;168mQ\u{1b}[38;2;170;173;169mQ\u{1b}[38;2;247;248;246m%\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;255;255;254m$\u{1b}[38;2;114;116;113mf\u{1b}[38;2;9;12;5m`\u{1b}[38;2;10;11;3m`\u{1b}[38;2;39;40;35ml\u{1b}[38;2;145;151;146mX\u{1b}[38;2;148;154;150mY\u{1b}[38;2;149;156;152mY\u{1b}[38;2;149;155;152mY\u{1b}[38;2;148;156;152mY\u{1b}[38;2;146;154;150mX\u{1b}[38;2;145;151;148mX\u{1b}[38;2;142;149;145mz\u{1b}[0m\n\u{1b}[38;2;13;14;15m^\u{1b}[38;2;18;19;19m\\\u{1b}[38;2;19;21;21m\\\u{1b}[38;2;18;20;21m\\\u{1b}[38;2;19;20;21m\\\u{1b}[38;2;20;22;24m\\\u{1b}[38;2;25;25;27m,\u{1b}[38;2;28;28;29m:\u{1b}[38;2;30;31;30m;\u{1b}[38;2;32;33;32m;\u{1b}[38;2;34;34;34mI\u{1b}[38;2;35;37;37mI\u{1b}[38;2;40;40;40ml\u{1b}[38;2;41;42;42m!\u{1b}[38;2;44;44;44m!\u{1b}[38;2;46;47;46mi\u{1b}[38;2;49;49;48m>\u{1b}[38;2;51;51;51m>\u{1b}[38;2;53;53;53m<\u{1b}[38;2;56;56;54m~\u{1b}[38;2;57;57;54m~\u{1b}[38;2;51;51;48m>\u{1b}[38;2;15;15;13m^\u{1b}[38;2;5;5;5m.\u{1b}[38;2;55;54;51m<\u{1b}[38;2;151;149;143mX\u{1b}[38;2;167;165;160mC\u{1b}[38;2;167;166;161mC\u{1b}[38;2;167;166;162mC\u{1b}[38;2;166;165;160mC\u{1b}[38;2;165;163;159mC\u{1b}[38;2;163;163;158mJ\u{1b}[38;2;162;164;160mC\u{1b}[38;2;162;165;162mC\u{1b}[38;2;162;166;162mC\u{1b}[38;2;161;165;163mC\u{1b}[38;2;160;167;166mC\u{1b}[38;2;162;167;167mC\u{1b}[38;2;163;168;167mL\u{1b}[38;2;165;168;166mL\u{1b}[38;2;168;170;168mL\u{1b}[38;2;171;173;169mQ\u{1b}[38;2;180;181;176mO\u{1b}[38;2;253;253;252m$\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;253;253;253m$\u{1b}[38;2;46;48;43mi\u{1b}[38;2;9;12;5m`\u{1b}[38;2;9;10;4m`\u{1b}[38;2;99;102;97m|\u{1b}[38;2;157;163;160mJ\u{1b}[38;2;163;169;165mL\u{1b}[38;2;164;170;166mL\u{1b}[38;2;164;170;167mL\u{1b}[38;2;162;169;166mL\u{1b}[38;2;159;168;163mC\u{1b}[38;2;156;164;158mJ\u{1b}[38;2;154;162;155mU\u{1b}[0m\n\u{1b}[38;2;12;14;16m^\u{1b}[38;2;17;17;19m\"\u{1b}[38;2;23;23;25m,\u{1b}[38;2;25;25;27m,\u{1b}[38;2;22;23;25m,\u{1b}[38;2;23;24;25m,\u{1b}[38;2;25;26;27m,\u{1b}[38;2;27;28;30m:\u{1b}[38;2;30;30;32m;\u{1b}[38;2;32;32;32m;\u{1b}[38;2;33;33;33m;\u{1b}[38;2;35;37;36mI\u{1b}[38;2;39;39;39ml\u{1b}[38;2;42;42;42m!\u{1b}[38;2;44;45;44mi\u{1b}[38;2;47;47;47mi\u{1b}[38;2;51;51;50m>\u{1b}[38;2;54;54;53m<\u{1b}[38;2;56;56;56m~\u{1b}[38;2;58;58;56m~\u{1b}[38;2;60;60;58m+\u{1b}[38;2;61;61;59m+\u{1b}[38;2;59;59;57m~\u{1b}[38;2;6;6;6m.\u{1b}[38;2;5;6;7m.\u{1b}[38;2;40;40;38ml\u{1b}[38;2;161;160;156mJ\u{1b}[38;2;166;165;159mC\u{1b}[38;2;167;165;161mC\u{1b}[38;2;166;165;160mC\u{1b}[38;2;165;165;160mC\u{1b}[38;2;164;165;160mC\u{1b}[38;2;162;165;162mC\u{1b}[38;2;162;167;161mC\u{1b}[38;2;164;168;164mL\u{1b}[38;2;164;169;166mL\u{1b}[38;2;164;170;165mL\u{1b}[38;2;163;168;164mC\u{1b}[38;2;166;169;166mL\u{1b}[38;2;167;169;166mL\u{1b}[38;2;170;170;167mL\u{1b}[38;2;173;173;170mQ\u{1b}[38;2;191;192;188mw\u{1b}[38;2;254;254;254m$\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;216;216;215ma\u{1b}[38;2;13;15;7m^\u{1b}[38;2;8;11;4m`\u{1b}[38;2;32;34;29m;\u{1b}[38;2;156;160;156mU\u{1b}[38;2;166;172;167mL\u{1b}[38;2;169;175;170mQ\u{1b}[38;2;170;176;172m0\u{1b}[38;2;170;177;171m0\u{1b}[38;2;171;177;172m0\u{1b}[38;2;170;178;170m0\u{1b}[38;2;168;175;169mQ\u{1b}[38;2;166;173;167mQ\u{1b}[0m\n\u{1b}[38;2;12;14;16m^\u{1b}[38;2;14;15;16m\"\u{1b}[38;2;16;17;18m\"\u{1b}[38;2;19;20;20m\\\u{1b}[38;2;22;22;23m\\\u{1b}[38;2;24;25;26m,\u{1b}[38;2;25;26;27m,\u{1b}[38;2;26;28;27m:\u{1b}[38;2;29;30;30m;\u{1b}[38;2;32;33;33m;\u{1b}[38;2;34;34;34mI\u{1b}[38;2;35;36;37mI\u{1b}[38;2;39;39;39ml\u{1b}[38;2;42;42;42m!\u{1b}[38;2;45;45;45mi\u{1b}[38;2;48;48;48mi\u{1b}[38;2;52;52;52m<\u{1b}[38;2;57;57;57m~\u{1b}[38;2;60;60;60m+\u{1b}[38;2;62;62;62m+\u{1b}[38;2;63;64;64m_\u{1b}[38;2;64;65;62m_\u{1b}[38;2;63;64;62m_\u{1b}[38;2;6;7;6m.\u{1b}[38;2;5;8;8m.\u{1b}[38;2;18;18;17m\"\u{1b}[38;2;161;160;155mJ\u{1b}[38;2;165;164;159mC\u{1b}[38;2;166;164;160mC\u{1b}[38;2;163;163;158mJ\u{1b}[38;2;162;163;157mJ\u{1b}[38;2;162;163;159mJ\u{1b}[38;2;162;166;160mC\u{1b}[38;2;162;167;161mC\u{1b}[38;2;163;168;163mC\u{1b}[38;2;168;173;167mQ\u{1b}[38;2;167;172;166mL\u{1b}[38;2;167;170;166mL\u{1b}[38;2;167;169;165mL\u{1b}[38;2;167;170;165mL\u{1b}[38;2;169;170;165mL\u{1b}[38;2;172;173;168mQ\u{1b}[38;2;208;210;206mk\u{1b}[38;2;255;255;255m@@@@\u{1b}[38;2;185;186;183mm\u{1b}[38;2;9;12;5m`\u{1b}[38;2;7;10;4m`\u{1b}[38;2;91;94;89m)\u{1b}[38;2;175;178;174m0\u{1b}[38;2;177;182;176mO\u{1b}[38;2;178;185;178mZ\u{1b}[38;2;178;185;179mZ\u{1b}[38;2;179;185;180mZ\u{1b}[38;2;178;185;178mZ\u{1b}[38;2;177;184;177mZ\u{1b}[38;2;178;185;178mZ\u{1b}[38;2;178;185;179mZ\u{1b}[0m\n\u{1b}[38;2;14;15;17m\"\u{1b}[38;2;16;16;16m\"\u{1b}[38;2;17;18;19m\"\u{1b}[38;2;21;22;23m\\\u{1b}[38;2;25;25;25m,\u{1b}[38;2;26;27;27m:\u{1b}[38;2;27;28;28m:\u{1b}[38;2;28;29;29m:\u{1b}[38;2;31;31;32m;\u{1b}[38;2;31;34;34m;\u{1b}[38;2;34;35;35mI\u{1b}[38;2;37;37;37ml\u{1b}[38;2;39;39;39ml\u{1b}[38;2;42;42;42m!\u{1b}[38;2;45;45;45mi\u{1b}[38;2;49;49;49m>\u{1b}[38;2;53;53;53m<\u{1b}[38;2;58;58;58m~\u{1b}[38;2;62;62;61m+\u{1b}[38;2;63;65;64m_\u{1b}[38;2;65;66;66m_\u{1b}[38;2;66;67;66m-\u{1b}[38;2;55;57;56m~\u{1b}[38;2;5;6;5m.\u{1b}[38;2;4;7;7m.\u{1b}[38;2;59;60;57m+\u{1b}[38;2;160;159;154mU\u{1b}[38;2;162;160;155mJ\u{1b}[38;2;162;162;156mJJ\u{1b}[38;2;161;162;156mJ\u{1b}[38;2;160;162;156mJ\u{1b}[38;2;161;164;159mJ\u{1b}[38;2;162;165;159mC\u{1b}[38;2;163;166;161mC\u{1b}[38;2;166;169;164mL\u{1b}[38;2;166;170;164mL\u{1b}[38;2;165;168;162mL\u{1b}[38;2;166;167;160mC\u{1b}[38;2;167;168;161mL\u{1b}[38;2;169;168;162mL\u{1b}[38;2;172;171;165mQ\u{1b}[38;2;226;224;223m*\u{1b}[38;2;255;255;254m$\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;186;187;183mm\u{1b}[38;2;7;12;4m`\u{1b}[38;2;5;8;4m.\u{1b}[38;2;106;111;108m/\u{1b}[38;2;183;188;183mm\u{1b}[38;2;184;189;184mm\u{1b}[38;2;185;190;184mm\u{1b}[38;2;186;191;185mw\u{1b}[38;2;183;191;184mm\u{1b}[38;2;184;191;184mm\u{1b}[38;2;184;191;185mm\u{1b}[38;2;183;190;184mm\u{1b}[38;2;182;189;184mm\u{1b}[0m\n\u{1b}[38;2;15;18;19m\"\u{1b}[38;2;17;17;19m\"\u{1b}[38;2;21;21;23m\\\u{1b}[38;2;30;31;31m;\u{1b}[38;2;38;39;38ml\u{1b}[38;2;37;38;38ml\u{1b}[38;2;34;35;35mI\u{1b}[38;2;32;33;33m;\u{1b}[38;2;33;34;34mI\u{1b}[38;2;34;36;35mI\u{1b}[38;2;35;37;36mI\u{1b}[38;2;38;38;38ml\u{1b}[38;2;40;41;40ml\u{1b}[38;2;43;43;43m!\u{1b}[38;2;46;47;46mi\u{1b}[38;2;50;50;50m>\u{1b}[38;2;54;54;54m<\u{1b}[38;2;59;59;58m~\u{1b}[38;2;61;61;60m+\u{1b}[38;2;63;64;63m_\u{1b}[38;2;65;65;65m_\u{1b}[38;2;64;67;66m_\u{1b}[38;2;43;44;43m!\u{1b}[38;2;5;6;5m.\u{1b}[38;2;4;5;6m.\u{1b}[38;2;123;121;117mj\u{1b}[38;2;157;156;151mU\u{1b}[38;2;159;158;152mU\u{1b}[38;2;160;160;153mJ\u{1b}[38;2;158;160;154mU\u{1b}[38;2;159;160;155mJJ\u{1b}[38;2;159;161;155mJ\u{1b}[38;2;159;161;156mJ\u{1b}[38;2;160;161;155mJ\u{1b}[38;2;164;166;158mC\u{1b}[38;2;165;167;160mC\u{1b}[38;2;165;165;158mC\u{1b}[38;2;165;164;157mC\u{1b}[38;2;167;165;157mC\u{1b}[38;2;168;167;158mC\u{1b}[38;2;173;170;161mL\u{1b}[38;2;239;239;236m&\u{1b}[38;2;255;255;255m@@@\u{1b}[38;2;254;254;254m$\u{1b}[38;2;112;113;109mt\u{1b}[38;2;3;8;4m.\u{1b}[38;2;5;8;4m.\u{1b}[38;2;103;106;104m\\\u{1b}[38;2;192;194;189mq\u{1b}[38;2;189;192;188mw\u{1b}[38;2;188;193;187mw\u{1b}[38;2;189;193;187mw\u{1b}[38;2;188;194;188mw\u{1b}[38;2;188;193;188mw\u{1b}[38;2;187;193;186mw\u{1b}[38;2;188;192;186mw\u{1b}[38;2;188;193;187mw\u{1b}[0m\n\u{1b}[38;2;25;27;28m:\u{1b}[38;2;28;30;29m:\u{1b}[38;2;26;28;28m:\u{1b}[38;2;35;37;36mI\u{1b}[38;2;45;47;46mi\u{1b}[38;2;49;51;50m>\u{1b}[38;2;49;50;49m>\u{1b}[38;2;45;47;46mi\u{1b}[38;2;46;48;47mi\u{1b}[38;2;44;46;45mi\u{1b}[38;2;41;43;42m!\u{1b}[38;2;41;42;41m!\u{1b}[38;2;42;43;43m!\u{1b}[38;2;45;46;45mi\u{1b}[38;2;48;48;48mi\u{1b}[38;2;50;51;49m>\u{1b}[38;2;53;55;54m<\u{1b}[38;2;58;58;58m~\u{1b}[38;2;60;60;60m+\u{1b}[38;2;63;63;65m_\u{1b}[38;2;62;63;64m+\u{1b}[38;2;61;63;65m+\u{1b}[38;2;28;30;30m:\u{1b}[38;2;3;5;4m.\u{1b}[38;2;4;6;5m.\u{1b}[38;2;69;68;65m-\u{1b}[38;2;74;72;69m?\u{1b}[38;2;71;69;65m-\u{1b}[38;2;69;66;63m-\u{1b}[38;2;64;61;58m+\u{1b}[38;2;59;56;53m~\u{1b}[38;2;53;52;49m<\u{1b}[38;2;49;47;45mi\u{1b}[38;2;44;43;40m!\u{1b}[38;2;39;39;35ml\u{1b}[38;2;34;33;29m;\u{1b}[38;2;31;30;26m;\u{1b}[38;2;26;26;22m,\u{1b}[38;2;28;26;23m:\u{1b}[38;2;35;34;31mI\u{1b}[38;2;41;40;37ml\u{1b}[38;2;53;50;47m>\u{1b}[38;2;85;84;82m}\u{1b}[38;2;96;95;94m)\u{1b}[38;2;103;102;102m|\u{1b}[38;2;108;107;107m/\u{1b}[38;2;122;122;120mj\u{1b}[38;2;35;34;29mI\u{1b}[38;2;3;7;5m.\u{1b}[38;2;4;7;5m.\u{1b}[38;2;62;62;59m+\u{1b}[38;2;183;182;176mZ\u{1b}[38;2;182;182;175mZ\u{1b}[38;2;186;187;180mm\u{1b}[38;2;192;192;187mw\u{1b}[38;2;191;192;187mw\u{1b}[38;2;190;192;186mw\u{1b}[38;2;189;191;185mww\u{1b}[38;2;190;191;185mw\u{1b}[0m\n\u{1b}[38;2;27;28;31m:\u{1b}[38;2;37;39;38ml\u{1b}[38;2;34;36;35mI\u{1b}[38;2;43;45;44m!\u{1b}[38;2;55;57;56m~\u{1b}[38;2;63;65;64m_\u{1b}[38;2;70;72;71m??\u{1b}[38;2;70;72;70m?\u{1b}[38;2;62;64;63m_\u{1b}[38;2;51;53;51m<\u{1b}[38;2;45;47;46mi\u{1b}[38;2;45;47;45mi\u{1b}[38;2;47;49;48m>\u{1b}[38;2;50;51;50m>\u{1b}[38;2;52;53;52m<\u{1b}[38;2;55;55;54m<\u{1b}[38;2;56;56;56m~\u{1b}[38;2;41;42;42m!\u{1b}[38;2;12;12;12m^\u{1b}[38;2;3;3;3m  \u{1b}[38;2;4;4;4m.\u{1b}[38;2;3;4;4m..\u{1b}[38;2;3;3;3m \u{1b}[38;2;4;4;4m.\u{1b}[38;2;15;15;13m^\u{1b}[38;2;25;25;25m,\u{1b}[38;2;2;2;2m \u{1b}[38;2;2;2;1m \u{1b}[38;2;1;3;1m \u{1b}[38;2;1;4;2m \u{1b}[38;2;2;4;2m \u{1b}[38;2;3;4;2m \u{1b}[38;2;3;4;4m.\u{1b}[38;2;75;79;80m[\u{1b}[38;2;106;110;111m/\u{1b}[38;2;107;111;112m/\u{1b}[38;2;101;108;108m\\\u{1b}[38;2;11;13;12m^\u{1b}[38;2;3;5;4m.\u{1b}[38;2;11;13;12m^\u{1b}[38;2;4;5;5m.\u{1b}[38;2;4;5;6m.\u{1b}[38;2;21;21;21m\\\u{1b}[38;2;115;116;115mf\u{1b}[38;2;92;92;91m1\u{1b}[38;2;58;58;58m~\u{1b}[38;2;7;8;8m`\u{1b}[38;2;4;4;6m..\u{1b}[38;2;4;3;4m \u{1b}[38;2;5;4;4m.\u{1b}[38;2;9;8;7m`\u{1b}[38;2;30;28;27m:\u{1b}[38;2;55;52;50m<\u{1b}[38;2;81;78;75m[\u{1b}[38;2;109;105;101m\\\u{1b}[38;2;135;134;128mu\u{1b}[0m\n\u{1b}[38;2;22;24;24m,\u{1b}[38;2;28;29;29m:\u{1b}[38;2;35;37;36mI\u{1b}[38;2;42;44;43m!\u{1b}[38;2;48;50;49m>\u{1b}[38;2;52;54;54m<\u{1b}[38;2;58;59;59m~\u{1b}[38;2;62;63;63m+\u{1b}[38;2;62;64;62m_\u{1b}[38;2;58;60;58m+\u{1b}[38;2;51;53;52m<\u{1b}[38;2;47;49;46m>\u{1b}[38;2;47;48;46mi\u{1b}[38;2;48;50;50m>\u{1b}[38;2;50;52;49m>\u{1b}[38;2;52;54;51m<\u{1b}[38;2;53;54;52m<\u{1b}[38;2;48;49;49m>\u{1b}[38;2;5;6;6m.\u{1b}[38;2;3;5;4m.\u{1b}[38;2;6;7;7m.\u{1b}[38;2;19;20;19m\\\u{1b}[38;2;4;4;4m.\u{1b}[38;2;3;3;3m \u{1b}[38;2;3;3;2m  \u{1b}[38;2;120;124;126mr\u{1b}[38;2;144;148;151mz\u{1b}[38;2;136;141;143mv\u{1b}[38;2;6;8;8m`\u{1b}[38;2;2;4;3m \u{1b}[38;2;1;5;4m \u{1b}[38;2;2;5;4m.\u{1b}[38;2;3;6;5m.\u{1b}[38;2;2;6;5m.\u{1b}[38;2;2;5;5m.\u{1b}[38;2;93;97;99m)\u{1b}[38;2;148;154;156mY\u{1b}[38;2;148;153;156mY\u{1b}[38;2;147;156;157mY\u{1b}[38;2;15;20;18m\"\u{1b}[38;2;2;6;3m.\u{1b}[38;2;2;6;5m.\u{1b}[38;2;3;6;5m.\u{1b}[38;2;3;5;4m.\u{1b}[38;2;15;16;14m\"\u{1b}[38;2;238;239;238m&\u{1b}[38;2;255;255;255m@\u{1b}[38;2;254;254;254m$\u{1b}[38;2;173;179;182mO\u{1b}[38;2;3;5;6m.\u{1b}[38;2;4;6;6m.\u{1b}[38;2;6;6;6m.\u{1b}[38;2;5;6;6m.\u{1b}[38;2;44;45;45mi\u{1b}[38;2;54;55;55m<\u{1b}[38;2;19;21;21m\\\u{1b}[38;2;2;2;2m \u{1b}[38;2;2;2;1m \u{1b}[38;2;2;2;2m \u{1b}[0m\n\u{1b}[38;2;25;26;28m:\u{1b}[38;2;36;38;41ml\u{1b}[38;2;31;34;34m;\u{1b}[38;2;33;34;36mI\u{1b}[38;2;42;43;46m!\u{1b}[38;2;47;49;53m>\u{1b}[38;2;49;51;52m>\u{1b}[38;2;52;54;56m<\u{1b}[38;2;54;56;55m<\u{1b}[38;2;49;51;50m>\u{1b}[38;2;44;47;46mii\u{1b}[38;2;44;48;48mi\u{1b}[38;2;46;49;49m>\u{1b}[38;2;49;52;51m>\u{1b}[38;2;52;54;52m<\u{1b}[38;2;54;56;52m<\u{1b}[38;2;50;53;50m<\u{1b}[38;2;19;20;20m\\\u{1b}[38;2;4;6;5m.\u{1b}[38;2;3;5;4m...\u{1b}[38;2;2;4;3m  \u{1b}[38;2;3;3;3m \u{1b}[38;2;115;119;120mf\u{1b}[38;2;145;148;151mz\u{1b}[38;2;147;151;154mX\u{1b}[38;2;17;21;21m\\\u{1b}[38;2;1;4;1m \u{1b}[38;2;0;5;2m \u{1b}[38;2;1;5;4m \u{1b}[38;2;3;6;5m..\u{1b}[38;2;2;5;4m.\u{1b}[38;2;63;66;67m_\u{1b}[38;2;148;153;155mYY\u{1b}[38;2;147;155;156mY\u{1b}[38;2;15;19;18m\"\u{1b}[38;2;1;6;3m.\u{1b}[38;2;2;5;4m.\u{1b}[38;2;2;6;5m..\u{1b}[38;2;14;15;15m^\u{1b}[38;2;250;250;250mB\u{1b}[38;2;255;255;255m@@\u{1b}[38;2;252;253;252m$\u{1b}[38;2;50;50;52m>\u{1b}[38;2;4;6;5m.\u{1b}[38;2;5;7;6m.\u{1b}[38;2;8;8;8m`\u{1b}[38;2;184;187;189mm\u{1b}[38;2;251;253;252m$\u{1b}[38;2;43;49;46mi\u{1b}[38;2;2;3;2m \u{1b}[38;2;2;4;2m \u{1b}[38;2;2;4;3m \u{1b}[0m\n\u{1b}[38;2;28;29;29m:\u{1b}[38;2;30;31;31m;\u{1b}[38;2;28;30;30m:\u{1b}[38;2;24;25;27m,\u{1b}[38;2;53;55;54m<\u{1b}[38;2;104;107;105m\\\u{1b}[38;2;145;147;143mz\u{1b}[38;2;207;208;203mk\u{1b}[38;2;212;213;209mh\u{1b}[38;2;134;140;138mv\u{1b}[38;2;40;46;46m!\u{1b}[38;2;40;45;45m!\u{1b}[38;2;42;47;47mi\u{1b}[38;2;44;49;50mi\u{1b}[38;2;47;51;50m>\u{1b}[38;2;52;54;53m<\u{1b}[38;2;55;57;54m~\u{1b}[38;2;56;59;56m~\u{1b}[38;2;60;61;59m+\u{1b}[38;2;25;27;26m:\u{1b}[38;2;4;6;5m...\u{1b}[38;2;2;4;3m \u{1b}[38;2;3;5;4m.\u{1b}[38;2;2;4;4m \u{1b}[38;2;114;117;118mf\u{1b}[38;2;146;152;152mX\u{1b}[38;2;150;157;158mY\u{1b}[38;2;34;40;38ml\u{1b}[38;2;0;5;1m   \u{1b}[38;2;3;6;4m.\u{1b}[38;2;2;4;3m \u{1b}[38;2;3;4;3m \u{1b}[38;2;50;53;54m<\u{1b}[38;2;147;152;155mX\u{1b}[38;2;145;150;152mX\u{1b}[38;2;145;151;153mX\u{1b}[38;2;13;18;15m\"\u{1b}[38;2;1;5;3m \u{1b}[38;2;2;5;4m.\u{1b}[38;2;2;6;5m.\u{1b}[38;2;3;6;5m.\u{1b}[38;2;21;21;20m\\\u{1b}[38;2;236;236;234mW\u{1b}[38;2;241;241;239m8\u{1b}[38;2;244;244;243m8\u{1b}[38;2;250;250;250mB\u{1b}[38;2;98;98;98m(\u{1b}[38;2;2;4;3m \u{1b}[38;2;3;5;4m.\u{1b}[38;2;23;24;23m,\u{1b}[38;2;157;157;151mU\u{1b}[38;2;149;151;146mX\u{1b}[38;2;9;12;10m`\u{1b}[38;2;0;5;1m \u{1b}[38;2;0;4;2m \u{1b}[38;2;1;5;3m \u{1b}[0m\n\u{1b}[38;2;23;24;27m,\u{1b}[38;2;24;26;25m,\u{1b}[38;2;22;25;24m,\u{1b}[38;2;11;12;14m^\u{1b}[38;2;23;23;24m,\u{1b}[38;2;71;73;69m?\u{1b}[38;2;138;139;135mv\u{1b}[38;2;191;192;186mw\u{1b}[38;2;191;191;187mw\u{1b}[38;2;130;130;128mn\u{1b}[38;2;49;48;47m>\u{1b}[38;2;29;30;31m;\u{1b}[38;2;36;40;43ml\u{1b}[38;2;43;47;48mi\u{1b}[38;2;47;49;51m>\u{1b}[38;2;52;54;53m<\u{1b}[38;2;55;57;55m~\u{1b}[38;2;57;59;55m~\u{1b}[38;2;59;61;59m+\u{1b}[38;2;24;28;25m:\u{1b}[38;2;2;7;2m.\u{1b}[38;2;1;6;3m.\u{1b}[38;2;2;6;3m.\u{1b}[38;2;3;5;5m.\u{1b}[38;2;2;6;5m.\u{1b}[38;2;2;5;4m.\u{1b}[38;2;110;115;112mt\u{1b}[38;2;149;154;152mY\u{1b}[38;2;153;159;158mU\u{1b}[38;2;46;53;50m>\u{1b}[38;2;0;4;1m \u{1b}[38;2;1;6;2m.\u{1b}[38;2;1;4;3m \u{1b}[38;2;7;9;7m`\u{1b}[38;2;3;4;4m.\u{1b}[38;2;2;2;2m \u{1b}[38;2;74;77;77m]\u{1b}[38;2;145;150;153mX\u{1b}[38;2;143;148;151mz\u{1b}[38;2;138;144;146mc\u{1b}[38;2;7;10;9m`\u{1b}[38;2;2;5;4m.\u{1b}[38;2;3;5;5m..\u{1b}[38;2;3;4;4m.\u{1b}[38;2;32;32;30m;\u{1b}[38;2;155;156;151mY\u{1b}[38;2;155;158;154mU\u{1b}[38;2;157;159;156mU\u{1b}[38;2;161;162;159mJ\u{1b}[38;2;56;58;55m~\u{1b}[38;2;2;3;2m \u{1b}[38;2;2;4;3m \u{1b}[38;2;18;19;18m\\\u{1b}[38;2;148;147;141mz\u{1b}[38;2;41;42;38m!\u{1b}[38;2;1;3;1m \u{1b}[38;2;2;4;2m \u{1b}[38;2;1;4;2m \u{1b}[38;2;1;5;4m \u{1b}[0m\n\u{1b}[38;2;23;25;26m,\u{1b}[38;2;17;19;19m\"\u{1b}[38;2;11;12;12m^\u{1b}[38;2;2;2;3m \u{1b}[38;2;2;2;5m \u{1b}[38;2;2;3;3m \u{1b}[38;2;7;8;7m`\u{1b}[38;2;16;16;15m\"\u{1b}[38;2;29;29;27m:\u{1b}[38;2;74;74;72m?\u{1b}[38;2;70;70;67m-\u{1b}[38;2;45;45;44mi\u{1b}[38;2;28;31;33m;\u{1b}[38;2;43;46;49mi\u{1b}[38;2;46;50;52m>\u{1b}[38;2;51;54;55m<\u{1b}[38;2;54;57;57m~\u{1b}[38;2;56;59;57m~\u{1b}[38;2;59;61;60m+\u{1b}[38;2;17;23;18m\\\u{1b}[38;2;2;7;2m.\u{1b}[38;2;2;6;3m.\u{1b}[38;2;1;6;3m.\u{1b}[38;2;3;5;4m.\u{1b}[38;2;2;6;5m.\u{1b}[38;2;2;4;4m \u{1b}[38;2;113;120;117mf\u{1b}[38;2;152;157;154mY\u{1b}[38;2;156;162;157mJ\u{1b}[38;2;45;48;44mi\u{1b}[38;2;1;5;1m \u{1b}[38;2;0;5;1m \u{1b}[38;2;2;4;3m  \u{1b}[38;2;2;3;3m \u{1b}[38;2;3;3;2m \u{1b}[38;2;78;79;79m[\u{1b}[38;2;146;149;153mX\u{1b}[38;2;142;147;150mz\u{1b}[38;2;133;139;141mv\u{1b}[38;2;5;6;8m.\u{1b}[38;2;3;4;4m.\u{1b}[38;2;3;4;6m..\u{1b}[38;2;2;3;4m \u{1b}[38;2;37;39;36ml\u{1b}[38;2;155;156;151mY\u{1b}[38;2;155;156;153mU\u{1b}[38;2;155;159;155mU\u{1b}[38;2;160;162;159mJ\u{1b}[38;2;40;43;42m!\u{1b}[38;2;2;3;3m  \u{1b}[38;2;15;16;14m\"\u{1b}[38;2;107;109;103m/\u{1b}[38;2;4;6;4m.\u{1b}[38;2;1;3;1m \u{1b}[38;2;2;4;3m \u{1b}[38;2;2;4;4m \u{1b}[38;2;1;5;4m \u{1b}[0m\n\u{1b}[38;2;25;26;25m,\u{1b}[38;2;8;11;10m`\u{1b}[38;2;3;3;3m \u{1b}[38;2;2;2;2m \u{1b}[38;2;1;2;2m \u{1b}[38;2;2;2;1m \u{1b}[38;2;6;5;5m.\u{1b}[38;2;19;20;19m\\\u{1b}[38;2;15;17;14m\"\u{1b}[38;2;7;7;6m.\u{1b}[38;2;11;11;10m`\u{1b}[38;2;6;6;6m.\u{1b}[38;2;36;40;41ml\u{1b}[38;2;42;47;50mi\u{1b}[38;2;46;51;54m>\u{1b}[38;2;49;55;55m<\u{1b}[38;2;53;57;58m~\u{1b}[38;2;55;59;58m~\u{1b}[38;2;57;60;59m~\u{1b}[38;2;9;14;11m^\u{1b}[38;2;1;6;2m.\u{1b}[38;2;2;6;2m.\u{1b}[38;2;1;5;2m \u{1b}[38;2;2;4;3m \u{1b}[38;2;1;6;3m.\u{1b}[38;2;1;5;3m \u{1b}[38;2;127;133;129mn\u{1b}[38;2;151;156;153mY\u{1b}[38;2;154;161;157mU\u{1b}[38;2;46;49;45mi\u{1b}[38;2;1;5;0m \u{1b}[38;2;0;6;1m \u{1b}[38;2;2;4;3m  \u{1b}[38;2;1;3;2m \u{1b}[38;2;2;3;1m \u{1b}[38;2;73;74;74m?\u{1b}[38;2;148;151;154mX\u{1b}[38;2;143;148;150mz\u{1b}[38;2;134;139;142mv\u{1b}[38;2;7;8;10m`\u{1b}[38;2;2;4;6m \u{1b}[38;2;3;4;4m.\u{1b}[38;2;2;4;4m \u{1b}[38;2;2;4;3m \u{1b}[38;2;19;21;19m\\\u{1b}[38;2;109;110;105m/\u{1b}[38;2;102;102;100m|\u{1b}[38;2;108;109;106m/\u{1b}[38;2;112;114;111mt\u{1b}[38;2;21;21;21m\\\u{1b}[38;2;2;3;6m \u{1b}[38;2;2;4;5m \u{1b}[38;2;4;4;5m.\u{1b}[38;2;25;28;26m:\u{1b}[38;2;2;3;4m \u{1b}[38;2;3;3;3m \u{1b}[38;2;1;3;2m \u{1b}[38;2;2;4;3m \u{1b}[38;2;1;4;6m \u{1b}[0m\n\u{1b}[38;2;21;24;23m,\u{1b}[38;2;7;8;8m`\u{1b}[38;2;2;3;3m \u{1b}[38;2;6;10;9m`\u{1b}[38;2;3;4;4m.\u{1b}[38;2;1;2;2m \u{1b}[38;2;2;3;2m \u{1b}[38;2;6;6;6m.\u{1b}[38;2;45;47;45mi\u{1b}[38;2;45;46;43mi\u{1b}[38;2;108;109;105m/\u{1b}[38;2;21;22;23m\\\u{1b}[38;2;41;44;47m!\u{1b}[38;2;46;50;53m>\u{1b}[38;2;50;54;56m<\u{1b}[38;2;51;56;57m<\u{1b}[38;2;54;58;60m~\u{1b}[38;2;55;60;59m~\u{1b}[38;2;57;59;58m~\u{1b}[38;2;11;16;13m^\u{1b}[38;2;0;6;3m.\u{1b}[38;2;0;5;1m \u{1b}[38;2;1;4;2m \u{1b}[38;2;1;3;2m \u{1b}[38;2;1;4;3m \u{1b}[38;2;4;4;4m.\u{1b}[38;2;122;126;121mr\u{1b}[38;2;149;155;149mY\u{1b}[38;2;151;158;153mU\u{1b}[38;2;61;66;61m_\u{1b}[38;2;1;4;1m \u{1b}[38;2;1;5;1m  \u{1b}[38;2;2;5;4m.\u{1b}[38;2;3;5;4m.\u{1b}[38;2;5;7;5m.\u{1b}[38;2;60;59;60m+\u{1b}[38;2;132;134;134mu\u{1b}[38;2;99;101;100m|\u{1b}[38;2;75;77;78m]\u{1b}[38;2;33;35;35mI\u{1b}[38;2;38;41;43ml\u{1b}[38;2;55;59;60m~\u{1b}[38;2;67;73;72m?\u{1b}[38;2;75;81;81m[\u{1b}[38;2;83;89;88m{\u{1b}[38;2;45;49;48mi\u{1b}[38;2;4;4;5m.\u{1b}[38;2;29;29;28m:\u{1b}[38;2;92;93;92m)\u{1b}[38;2;26;26;27m:\u{1b}[38;2;4;5;7m.\u{1b}[38;2;4;4;5m.\u{1b}[38;2;4;5;7m.\u{1b}[38;2;3;5;5m.\u{1b}[38;2;2;4;4m \u{1b}[38;2;2;4;3m   \u{1b}[38;2;1;4;6m \u{1b}[0m\n\u{1b}[38;2;24;26;25m,\u{1b}[38;2;3;4;4m.\u{1b}[38;2;2;2;3m \u{1b}[38;2;2;3;2m \u{1b}[38;2;2;4;3m \u{1b}[38;2;1;2;1m \u{1b}[38;2;40;41;40ml\u{1b}[38;2;19;20;19m\\\u{1b}[38;2;2;2;2m \u{1b}[38;2;5;5;4m.\u{1b}[38;2;28;28;27m:\u{1b}[38;2;43;44;46m!\u{1b}[38;2;47;49;52m>\u{1b}[38;2;52;54;55m<\u{1b}[38;2;54;57;57m~\u{1b}[38;2;56;59;57m~\u{1b}[38;2;58;60;59m+\u{1b}[38;2;58;60;58m+\u{1b}[38;2;59;61;58m+\u{1b}[38;2;24;26;24m,\u{1b}[38;2;2;4;3m \u{1b}[38;2;2;4;4m \u{1b}[38;2;2;4;3m \u{1b}[38;2;1;3;2m \u{1b}[38;2;2;4;3m \u{1b}[38;2;2;3;3m \u{1b}[38;2;116;121;117mj\u{1b}[38;2;149;154;148mY\u{1b}[38;2;150;156;150mY\u{1b}[38;2;95;99;94m(\u{1b}[38;2;2;4;2m \u{1b}[38;2;1;7;4m.\u{1b}[38;2;2;7;3m.\u{1b}[38;2;3;6;5m.\u{1b}[38;2;7;8;6m`\u{1b}[38;2;37;39;38ml\u{1b}[38;2;55;57;56m~\u{1b}[38;2;83;87;86m{\u{1b}[38;2;116;121;120mj\u{1b}[38;2;139;146;145mc\u{1b}[38;2;146;156;156mY\u{1b}[38;2;146;156;155mY\u{1b}[38;2;147;156;155mY\u{1b}[38;2;147;157;154mY\u{1b}[38;2;147;157;155mY\u{1b}[38;2;150;159;158mU\u{1b}[38;2;96;104;100m|\u{1b}[38;2;8;9;7m`\u{1b}[38;2;42;44;41m!\u{1b}[38;2;189;190;186mw\u{1b}[38;2;188;188;186mm\u{1b}[38;2;179;179;179mO\u{1b}[38;2;161;162;161mJ\u{1b}[38;2;187;188;187mm\u{1b}[38;2;146;150;152mX\u{1b}[38;2;58;62;64m+\u{1b}[38;2;4;8;8m.\u{1b}[38;2;2;6;6m.\u{1b}[38;2;2;6;7m.\u{1b}[38;2;1;6;7m.\u{1b}[0m\n\u{1b}[38;2;29;30;32m;\u{1b}[38;2;9;10;10m`\u{1b}[38;2;7;8;7m`\u{1b}[38;2;2;3;2m \u{1b}[38;2;2;4;3m \u{1b}[38;2;2;3;2m \u{1b}[38;2;35;37;37mI\u{1b}[38;2;45;47;48mi\u{1b}[38;2;27;28;29m:\u{1b}[38;2;36;36;37mI\u{1b}[38;2;4;5;5m.\u{1b}[38;2;40;41;40ml\u{1b}[38;2;52;53;55m<\u{1b}[38;2;55;57;57m~\u{1b}[38;2;58;60;59m+\u{1b}[38;2;60;62;59m+\u{1b}[38;2;60;61;59m+\u{1b}[38;2;60;61;58m+\u{1b}[38;2;61;62;60m+\u{1b}[38;2;53;53;51m<\u{1b}[38;2;4;6;4m.\u{1b}[38;2;2;4;4m \u{1b}[38;2;2;4;3m \u{1b}[38;2;1;3;1m \u{1b}[38;2;1;2;1m \u{1b}[38;2;2;4;3m \u{1b}[38;2;59;61;59m+\u{1b}[38;2;72;74;71m?\u{1b}[38;2;45;46;45mi\u{1b}[38;2;23;23;23m,\u{1b}[38;2;24;25;26m,\u{1b}[38;2;52;54;54m<\u{1b}[38;2;81;84;85m}\u{1b}[38;2;116;120;120mj\u{1b}[38;2;150;153;153mY\u{1b}[38;2;158;162;160mJ\u{1b}[38;2;157;161;160mJ\u{1b}[38;2;157;162;163mJ\u{1b}[38;2;155;162;161mJ\u{1b}[38;2;153;162;160mJ\u{1b}[38;2;152;160;159mU\u{1b}[38;2;152;161;160mU\u{1b}[38;2;152;161;159mU\u{1b}[38;2;154;163;161mJJ\u{1b}[38;2;196;201;197mp\u{1b}[38;2;169;171;168mQ\u{1b}[38;2;8;12;9m`\u{1b}[38;2;36;38;36ml\u{1b}[38;2;186;185;181mm\u{1b}[38;2;183;183;179mZ\u{1b}[38;2;184;184;182mZ\u{1b}[38;2;185;186;183mm\u{1b}[38;2;186;187;186mm\u{1b}[38;2;187;188;186mm\u{1b}[38;2;188;191;189mw\u{1b}[38;2;160;163;163mJ\u{1b}[38;2;71;76;78m]\u{1b}[38;2;6;10;11m`\u{1b}[38;2;46;50;53m>\u{1b}[0m\n\u{1b}[38;2;27;29;30m:\u{1b}[38;2;8;9;9m`\u{1b}[38;2;5;6;5m.\u{1b}[38;2;1;3;2m \u{1b}[38;2;2;3;3m \u{1b}[38;2;12;13;12m^\u{1b}[38;2;51;54;53m<\u{1b}[38;2;77;78;78m[\u{1b}[38;2;126;126;126mx\u{1b}[38;2;60;61;62m+\u{1b}[38;2;42;43;43m!\u{1b}[38;2;53;55;55m<\u{1b}[38;2;54;58;58m~\u{1b}[38;2;58;61;60m+\u{1b}[38;2;62;63;63m+\u{1b}[38;2;64;66;63m_\u{1b}[38;2;66;67;64m__\u{1b}[38;2;66;66;64m__\u{1b}[38;2;32;32;32m;\u{1b}[38;2;3;5;5m.\u{1b}[38;2;4;6;5m.\u{1b}[38;2;3;5;4m.\u{1b}[38;2;2;3;2m \u{1b}[38;2;17;19;18m\"\u{1b}[38;2;57;61;60m+\u{1b}[38;2;97;102;101m|\u{1b}[38;2;130;136;137mu\u{1b}[38;2;157;161;161mJ\u{1b}[38;2;161;166;167mC\u{1b}[38;2;162;165;166mC\u{1b}[38;2;163;166;167mC\u{1b}[38;2;162;165;165mC\u{1b}[38;2;160;164;162mJ\u{1b}[38;2;162;166;165mC\u{1b}[38;2;162;166;166mC\u{1b}[38;2;160;166;166mC\u{1b}[38;2;159;166;165mC\u{1b}[38;2;155;164;163mJ\u{1b}[38;2;156;164;163mJ\u{1b}[38;2;155;165;164mJ\u{1b}[38;2;158;166;162mC\u{1b}[38;2;159;167;161mC\u{1b}[38;2;223;226;223m*\u{1b}[38;2;254;254;252m$\u{1b}[38;2;178;179;176mO\u{1b}[38;2;10;13;10m^\u{1b}[38;2;42;45;42m!\u{1b}[38;2;178;178;173mO\u{1b}[38;2;178;179;174mO\u{1b}[38;2;179;180;175mO\u{1b}[38;2;181;182;178mZ\u{1b}[38;2;183;183;181mZZ\u{1b}[38;2;182;183;181mZ\u{1b}[38;2;180;183;179mZ\u{1b}[38;2;181;183;181mZ\u{1b}[38;2;155;157;155mU\u{1b}[38;2;80;81;82m[\u{1b}[0m\n\u{1b}[38;2;25;28;27m:\u{1b}[38;2;15;15;15m\"\u{1b}[38;2;2;2;2m \u{1b}[38;2;2;3;3m \u{1b}[38;2;13;14;13m^\u{1b}[38;2;87;91;91m1\u{1b}[38;2;53;55;55m<\u{1b}[38;2;39;41;39ml\u{1b}[38;2;154;155;156mY\u{1b}[38;2;44;47;46mi\u{1b}[38;2;53;55;58m<\u{1b}[38;2;56;61;61m+\u{1b}[38;2;57;62;62m+\u{1b}[38;2;59;63;63m+\u{1b}[38;2;62;66;66m_\u{1b}[38;2;67;69;67m-\u{1b}[38;2;71;73;70m?\u{1b}[38;2;74;75;72m]\u{1b}[38;2;75;75;72m]\u{1b}[38;2;74;75;70m]\u{1b}[38;2;70;71;67m?\u{1b}[38;2;15;18;15m\"\u{1b}[38;2;8;10;9m`\u{1b}[38;2;24;28;26m:\u{1b}[38;2;118;124;122mr\u{1b}[38;2;152;159;157mU\u{1b}[38;2;154;160;159mU\u{1b}[38;2;155;162;160mJ\u{1b}[38;2;158;163;161mJ\u{1b}[38;2;160;163;164mJ\u{1b}[38;2;160;165;165mC\u{1b}[38;2;161;166;165mC\u{1b}[38;2;161;165;164mCC\u{1b}[38;2;162;166;165mC\u{1b}[38;2;164;167;165mC\u{1b}[38;2;165;168;166mL\u{1b}[38;2;162;167;165mC\u{1b}[38;2;160;166;164mC\u{1b}[38;2;159;168;166mC\u{1b}[38;2;161;169;169mL\u{1b}[38;2;160;170;164mL\u{1b}[38;2;173;181;175mO\u{1b}[38;2;235;236;232mW\u{1b}[38;2;254;255;252m$\u{1b}[38;2;255;255;253m$\u{1b}[38;2;156;157;154mU\u{1b}[38;2;11;15;13m^\u{1b}[38;2;60;62;61m+\u{1b}[38;2;167;169;164mL\u{1b}[38;2;169;170;165mL\u{1b}[38;2;173;174;170mQ\u{1b}[38;2;174;176;171m0\u{1b}[38;2;177;177;175m0\u{1b}[38;2;176;177;175m0\u{1b}[38;2;175;177;174m0\u{1b}[38;2;174;177;174m0\u{1b}[38;2;171;175;171mQ\u{1b}[38;2;169;174;169mQ\u{1b}[38;2;168;173;169mQ\u{1b}[0m\n\u{1b}[38;2;36;38;35ml\u{1b}[38;2;26;27;26m:\u{1b}[38;2;22;23;22m,\u{1b}[38;2;29;28;28m:\u{1b}[38;2;71;73;74m?\u{1b}[38;2;122;125;129mr\u{1b}[38;2;159;162;166mJ\u{1b}[38;2;106;107;108m\\\u{1b}[38;2;91;92;92m1\u{1b}[38;2;52;57;57m~\u{1b}[38;2;55;61;61m+\u{1b}[38;2;58;64;64m+\u{1b}[38;2;59;66;65m_\u{1b}[38;2;61;67;67m_\u{1b}[38;2;66;69;69m-\u{1b}[38;2;70;74;70m?\u{1b}[38;2;75;77;75m]\u{1b}[38;2;78;79;74m[[\u{1b}[38;2;78;79;75m[\u{1b}[38;2;78;79;74m[\u{1b}[38;2;42;45;40m!\u{1b}[38;2;7;10;6m`\u{1b}[38;2;47;52;51m>\u{1b}[38;2;157;163;160mJ\u{1b}[38;2;158;162;159mJ\u{1b}[38;2;160;164;161mJ\u{1b}[38;2;159;164;160mJ\u{1b}[38;2;162;166;163mC\u{1b}[38;2;163;166;164mC\u{1b}[38;2;162;166;162mC\u{1b}[38;2;161;165;163mCC\u{1b}[38;2;161;165;162mCC\u{1b}[38;2;167;171;169mL\u{1b}[38;2;167;171;170mL\u{1b}[38;2;164;168;167mL\u{1b}[38;2;161;166;164mC\u{1b}[38;2;158;166;164mC\u{1b}[38;2;161;169;166mC\u{1b}[38;2;168;175;171mQ\u{1b}[38;2;247;248;244m%\u{1b}[38;2;254;255;250m$\u{1b}[38;2;255;255;252m$\u{1b}[38;2;255;255;254m$\u{1b}[38;2;138;140;136mv\u{1b}[38;2;12;16;13m^\u{1b}[38;2;84;88;85m{\u{1b}[38;2;159;161;156mJ\u{1b}[38;2;162;164;160mC\u{1b}[38;2;165;167;164mC\u{1b}[38;2;166;169;165mL\u{1b}[38;2;166;170;166mL\u{1b}[38;2;166;170;165mL\u{1b}[38;2;165;170;164mL\u{1b}[38;2;164;170;164mL\u{1b}[38;2;164;169;163mL\u{1b}[38;2;161;168;162mC\u{1b}[38;2;160;167;161mC\u{1b}[0m\n\u{1b}[38;2;42;44;43m!\u{1b}[38;2;43;44;42m!\u{1b}[38;2;40;42;40m!\u{1b}[38;2;23;24;24m,\u{1b}[38;2;4;3;5m \u{1b}[38;2;4;4;4m.\u{1b}[38;2;30;32;31m;\u{1b}[38;2;51;53;52m<\u{1b}[38;2;55;57;58m~\u{1b}[38;2;56;60;61m~\u{1b}[38;2;58;63;64m+\u{1b}[38;2;61;67;67m_\u{1b}[38;2;64;68;69m-\u{1b}[38;2;67;71;70m-\u{1b}[38;2;70;74;71m?\u{1b}[38;2;73;77;73m]\u{1b}[38;2;79;79;77m[\u{1b}[38;2;80;80;77m[\u{1b}[38;2;80;81;77m[\u{1b}[38;2;80;80;78m[\u{1b}[38;2;81;82;78m[\u{1b}[38;2;65;64;60m_\u{1b}[38;2;11;12;5m`\u{1b}[38;2;35;37;33mI\u{1b}[38;2;159;162;159mJ\u{1b}[38;2;161;164;161mC\u{1b}[38;2;163;167;163mCC\u{1b}[38;2;162;165;162mC\u{1b}[38;2;163;165;162mC\u{1b}[38;2;161;163;160mJ\u{1b}[38;2;159;162;159mJ\u{1b}[38;2;162;166;163mC\u{1b}[38;2;161;165;163mC\u{1b}[38;2;160;166;162mC\u{1b}[38;2;163;168;166mC\u{1b}[38;2;167;171;169mL\u{1b}[38;2;165;169;167mL\u{1b}[38;2;161;166;164mC\u{1b}[38;2;159;167;164mC\u{1b}[38;2;158;167;163mC\u{1b}[38;2;163;171;166mL\u{1b}[38;2;251;252;249m$\u{1b}[38;2;255;255;254m$$\u{1b}[38;2;255;255;255m@\u{1b}[38;2;110;113;110mt\u{1b}[38;2;10;16;14m^\u{1b}[38;2;89;92;88m1\u{1b}[38;2;149;151;147mX\u{1b}[38;2;150;154;149mY\u{1b}[38;2;152;157;151mY\u{1b}[38;2;154;159;153mU\u{1b}[38;2;155;160;155mU\u{1b}[38;2;156;161;155mUU\u{1b}[38;2;153;160;154mU\u{1b}[38;2;152;158;153mU\u{1b}[38;2;150;156;151mY\u{1b}[38;2;149;156;149mY\u{1b}[0m\n\u{1b}[38;2;47;49;46m>\u{1b}[38;2;47;49;47m>\u{1b}[38;2;45;48;47mi\u{1b}[38;2;22;24;24m,\u{1b}[38;2;4;4;6m.\u{1b}[38;2;5;6;6m.\u{1b}[38;2;46;48;48mi\u{1b}[38;2;55;58;60m~\u{1b}[38;2;58;61;62m+\u{1b}[38;2;59;63;63m+\u{1b}[38;2;60;64;65m_\u{1b}[38;2;62;66;67m_\u{1b}[38;2;65;69;69m-\u{1b}[38;2;69;72;71m?\u{1b}[38;2;72;75;71m?\u{1b}[38;2;75;78;74m]\u{1b}[38;2;80;80;78m[\u{1b}[38;2;81;81;79m[\u{1b}[38;2;82;82;81m}\u{1b}[38;2;82;82;79m}\u{1b}[38;2;82;83;81m}\u{1b}[38;2;59;59;56m~\u{1b}[38;2;14;14;8m^\u{1b}[38;2;10;11;5m`\u{1b}[38;2;146;147;143mz\u{1b}[38;2;164;165;164mC\u{1b}[38;2;166;166;164mC\u{1b}[38;2;165;166;164mC\u{1b}[38;2;165;167;163mC\u{1b}[38;2;164;166;163mC\u{1b}[38;2;162;164;161mC\u{1b}[38;2;160;162;159mJ\u{1b}[38;2;159;161;158mJ\u{1b}[38;2;159;162;159mJ\u{1b}[38;2;161;165;162mC\u{1b}[38;2;163;168;164mC\u{1b}[38;2;170;174;171mQ\u{1b}[38;2;165;169;168mL\u{1b}[38;2;160;166;162mC\u{1b}[38;2;161;167;165mC\u{1b}[38;2;163;170;165mL\u{1b}[38;2;169;177;172m0\u{1b}[38;2;253;253;251m$\u{1b}[38;2;255;255;254m$\u{1b}[38;2;255;255;255m@@\u{1b}[38;2;86;89;84m{\u{1b}[38;2;11;17;12m^\u{1b}[38;2;104;107;101m\\\u{1b}[38;2;139;141;137mv\u{1b}[38;2;142;145;140mc\u{1b}[38;2;143;148;143mz\u{1b}[38;2;146;151;146mX\u{1b}[38;2;147;152;148mX\u{1b}[38;2;147;153;149mX\u{1b}[38;2;148;154;150mY\u{1b}[38;2;147;153;149mX\u{1b}[38;2;144;151;146mX\u{1b}[38;2;141;147;143mz\u{1b}[38;2;138;145;140mc\u{1b}[0m";

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub name: String,
    pub last_played: Option<Song>,
    pub songs_played: usize,
    pub image_url: Option<String>,
    pub pfp: String,
    pub time_played: usize,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            name: hostname().unwrap_or("username".to_string()),
            pfp: String::from(DEFAULT_PFP),
            image_url: None,
            last_played: None,
            songs_played: 0,
            time_played: 0,
        }
    }
}

#[derive(Error, Debug)]
pub enum UserProfileError {
    #[error("Database error: {0}")]
    DbError(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
    #[error("Image File Not Found")]
    ImageFileUrlNotFound,
    #[error("Cannot Convert Image to Ascii")]
    RenderFailed,
}

pub struct UserProfileDb {
    db: sled::Db,
}

impl UserProfileDb {
    pub fn new() -> Result<Self, UserProfileError> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push("Feather/user_profile");
        let db = sled::open(path)?;
        if db.get("user")?.is_none() {
            db.insert("user", bincode::serialize(&UserProfile::default())?);
        }
        Ok(Self { db })
    }

    pub fn add_time(&self) -> Result<(), UserProfileError> {
        let user = self.db.get("user")?.unwrap();
        let mut user_data: UserProfile = bincode::deserialize(&user)?;

        user_data.time_played += 1;

        let new_data = bincode::serialize(&user_data)?;
        self.db.insert("user", new_data)?;
        Ok(())
    }
    pub fn check_pfp_change(&self, config: Rc<USERCONFIG>) -> Result<(), UserProfileError> {
        let user = self.db.get("user")?.unwrap();
        let mut user_data: UserProfile = bincode::deserialize(&user)?;

        if let Some(image_url) = config.image_url.clone() {
            debug!("{:?}", image_url);
            let should_update = match &user_data.image_url {
                Some(last_image_url) => last_image_url != &image_url,
                None => true,
            };

            if should_update {
                if !Path::new(&image_url).is_file() {
                    return Err(UserProfileError::ImageFileUrlNotFound);
                }

                let mut buffer = String::new();
                render_to(
                    image_url.clone(),
                    &mut buffer,
                    &RenderOptions::new().width(80).height(25).colored(false),
                )
                .map_err(|_| UserProfileError::RenderFailed)?;

                user_data.image_url = Some(image_url);
                user_data.pfp = buffer;
            }
        }

        let new_data = bincode::serialize(&user_data)?;
        self.db.insert("user", new_data)?;
        Ok(())
    }
    pub fn set_last_played(&self, song: Song) -> Result<(), UserProfileError> {
        let user = self.db.get("user")?.unwrap();
        let mut user_data: UserProfile = bincode::deserialize(&user)?;

        user_data.last_played = Some(song);
        let new_data = bincode::serialize(&user_data)?;
        self.db.insert("user", new_data)?;
        Ok(())
    }

    pub fn add_song(&self) -> Result<(), UserProfileError> {
        let user = self.db.get("user")?.unwrap();
        let mut user_data: UserProfile = bincode::deserialize(&user)?;

        user_data.songs_played += 1;
        let new_data = bincode::serialize(&user_data)?;
        self.db.insert("user", new_data)?;
        Ok(())
    }

    pub fn give_info(&self) -> Result<UserProfile, UserProfileError> {
        let user = self.db.get("user")?.unwrap();
        debug!("{}", "user found");
        let mut user_data: UserProfile = bincode::deserialize(&user)?;
        Ok(user_data)
    }
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
