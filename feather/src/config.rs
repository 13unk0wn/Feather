#![allow(unused)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct USERCONFIG {
    pub bg_color: (u8, u8, u8),
    pub text_color: (u8, u8, u8),
    pub play_icon: String,
    pub pause_icon: String,
    pub selected_list_item: (u8, u8, u8),
    pub selected_item_char: String,
    pub selected_tab_color: (u8, u8, u8),
    pub player_progress_bar_color: (u8, u8, u8),
    pub player_volume_bar_color: (u8, u8, u8),
    pub selected_mode_text_color: (u8, u8, u8),
}

#[derive(Error, Debug)]
pub enum USERCONFIGERROR {
    #[error("VALID CONFIG")]
    ValidInputError,
    #[error("IO ERROR :  {0}")]
    IOERROR(#[from] std::io::Error),
}

impl Default for USERCONFIG {
    fn default() -> Self {
        Self {
            bg_color: (29, 32, 33),
            text_color: (235, 219, 178),
            play_icon: "▶".to_string(),
            pause_icon: "❚❚".to_string(),
            selected_list_item: (60, 56, 54),
            selected_item_char: '>'.to_string(),
            selected_tab_color: (250, 189, 47),
            player_progress_bar_color: (214, 93, 14),
            player_volume_bar_color: (152, 151, 26),
            selected_mode_text_color: (152, 151, 26),
        }
    }
}

impl USERCONFIG {
    pub fn new() -> Result<Self, USERCONFIGERROR> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push("Feather/config.toml");

        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let config: USERCONFIG =
                toml::from_str(&contents).map_err(|_| USERCONFIGERROR::ValidInputError)?;
            return Ok(config);
        } else {
            let default_config = USERCONFIG::default();
            let toml_str = toml::to_string_pretty(&default_config).unwrap();
            fs::write(&path, toml_str)?;
            return Ok(default_config);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyConfig {
    pub leader: char,
    pub navigation: Navigation,
    pub history: HistoryKeyBindings,
    pub default: DefaultControl,
    pub search: SearchKeyBindings,
    pub player: PlayerKeyBindings,
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            leader: ':',
            navigation: Navigation::default(),
            history: HistoryKeyBindings::default(),
            default: DefaultControl::default(),
            search: SearchKeyBindings::default(),
            player: PlayerKeyBindings::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Navigation {
    pub home: char,
    pub quit: char,
    pub search: char,
    pub player: char,
    pub history: char,
    pub userplaylist: char,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            home: ';',
            quit: 'q',
            search: 's',
            player: 'p',
            history: 'h',
            userplaylist: 'u',
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DefaultControl {
    pub up: char,
    pub down: char,
    pub next_page: char,
    pub prev_page: char,
    pub add_to_playlist: char,
    pub play_song: char,
}

impl Default for DefaultControl {
    fn default() -> Self {
        Self {
            up: 'k',
            down: 'j',
            next_page: 'l',
            prev_page: 'h',
            add_to_playlist: 'a',
            play_song: 'p',
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryKeyBindings {
    pub up: Option<char>,
    pub down: Option<char>,
    pub next: Option<char>,
    pub prev: Option<char>,
    pub add_to_playlist: Option<char>,
    pub play_song: Option<char>,
    // TODO :  Add delete
}

impl Default for HistoryKeyBindings {
    // will follow the default key bindigs
    fn default() -> Self {
        Self {
            up: None,
            down: None,
            next: None,
            prev: None,
            add_to_playlist: None,
            play_song: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchKeyBindings {
    pub switch: char, // allow to toggle b/w playlist and song search.
    pub up: Option<char>,
    pub down: Option<char>,
    pub playlist: PlaylistKeyBindings,
    pub song: SongSearchKeyBinding,
}

impl Default for SearchKeyBindings {
    fn default() -> Self {
        Self {
            switch: ';',
            up: None,
            down: None,
            playlist: PlaylistKeyBindings::default(),
            song: SongSearchKeyBinding::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaylistKeyBindings {
    pub switch_mode: char, // Switch Mode b/w ViewPlaylistSearch and SearchPlaylist
    pub playlist_search: PlayListSearchKeyBindings,
    pub view_playlist: PlayListViewKeyBindings,
}

impl Default for PlaylistKeyBindings {
    fn default() -> Self {
        Self {
            switch_mode: '[',
            playlist_search: PlayListSearchKeyBindings::default(),
            view_playlist: PlayListViewKeyBindings::default(),
        }
    }
}
// USERPLAYLIST WILL ALSO FOLLOW THE SAME KEYBINDINGS
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayListSearchKeyBindings {
    pub switch_mode: Option<char>, // it switch from playlist_search_box to playlist_view_box
    pub select_playlist: Option<char>,
}

impl Default for PlayListSearchKeyBindings {
    fn default() -> Self {
        Self {
            switch_mode: None,
            select_playlist: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayListViewKeyBindings {
    pub start_playlist: char,  // these key is used start playlist from song one
    pub start_from_here: char, // these key is used start song from the selected song
    pub next_page: Option<char>,
    pub prev_page: Option<char>,
}

impl Default for PlayListViewKeyBindings {
    fn default() -> Self {
        Self {
            start_playlist: 'p',
            start_from_here: 'a',
            next_page: None,
            prev_page: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SongSearchKeyBinding {
    pub switch_mode: Option<char>, // switch from song textbox to song search list
}

impl Default for SongSearchKeyBinding {
    fn default() -> Self {
        Self { switch_mode: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerKeyBindings {
    pub pause: char, // space will always work
    pub skip_plus_secs: char,
    pub skip_minus_secs: char,
    pub playlist_next_song: char,
    pub playlist_prev_song: char,
    pub volume_up: char,
    pub volume_down: char,
}

impl Default for PlayerKeyBindings {
    fn default() -> Self {
        Self {
            pause: ';',
            skip_plus_secs: 'l',
            skip_minus_secs: 'l',
            playlist_next_song: 'n',
            playlist_prev_song: 'p',
            volume_up: '+',
            volume_down: '-',
        }
    }
}

impl KeyConfig {
    pub fn new() -> Result<Self, USERCONFIGERROR> {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        path.push("Feather/keystrokes.toml");
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let config: KeyConfig =
                toml::from_str(&contents).map_err(|_| USERCONFIGERROR::ValidInputError)?;
            return Ok(config);
        } else {
            let default_config = KeyConfig::default();
            let toml_str = toml::to_string_pretty(&default_config).unwrap();
            fs::write(&path, toml_str)?;
            return Ok(default_config);
        }
    }
}
