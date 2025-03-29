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
struct KeyConfig {
    leader: char,
    navigation: Navigation,
    history: HistoryKeyBindings,
    default: DefaultControl,
    search: SearchKeyBindings,
    player: PlayerKeyBindings,
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
struct Navigation {
    home: char,
    quit: char,
    search: char,
    player: char,
    history: char,
    userplaylist: char,
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
struct DefaultControl {
    up: char,
    down: char,
    add_to_playlist: char,
    play_song: char,
}

impl Default for DefaultControl {
    fn default() -> Self {
        Self {
            up: 'k',
            down: 'j',
            add_to_playlist: 'a',
            play_song: 'p',
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct HistoryKeyBindings {
    up: Option<char>,
    down: Option<char>,
    add_to_playlist: Option<char>,
    play_song: Option<char>,
}

impl Default for HistoryKeyBindings {
    // will follow the default key bindigs
    fn default() -> Self {
        Self {
            up: None,
            down: None,
            add_to_playlist: None,
            play_song: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SearchKeyBindings {
    switch: char, // allow to toggle b/w playlist and song search.
    up: Option<char>,
    down: Option<char>,
    playlist: PlaylistKeyBindings,
    song: SongSearchKeyBinding,
}

impl Default for SearchKeyBindings {
    fn default() -> Self {
        Self {
            switch: ':',
            up: None,
            down: None,
            playlist: PlaylistKeyBindings::default(),
            song: SongSearchKeyBinding::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PlaylistKeyBindings {
    switch_mode: char, // Switch Mode b/w ViewPlaylistSearch and SearchPlaylist

    playlist_search: PlayListSearchKeyBindings,
    view_playlist: PlayListViewKeyBindings,
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
struct PlayListSearchKeyBindings {
    switch_mode: Option<char>, // it switch from playlist_search_box to playlist_view_box
    select_playlist: Option<char>,
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
struct PlayListViewKeyBindings {
    start_playlist: char,  // these key is used start playlist from song one
    start_from_here: char, // these key is used start song from the selected song
}

impl Default for PlayListViewKeyBindings {
    fn default() -> Self {
        Self {
            start_playlist: 'p',
            start_from_here: 'a',
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SongSearchKeyBinding {
    switch_mode: Option<char>, // switch from song textbox to song search list
}

impl Default for SongSearchKeyBinding {
    fn default() -> Self {
        Self { switch_mode: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PlayerKeyBindings {
    pause: char, // space will always work
    skip_plus_secs: char,
    skip_minus_secs: char,
    playlist_next_song: char,
    playlist_prev_song: char,
    volume_up: char,
    volume_down: char,
}

impl Default for PlayerKeyBindings {
    fn default() -> Self {
        Self {
            pause: 'p',
            skip_plus_secs: 'l',
            skip_minus_secs: 'l',
            playlist_next_song: 'n',
            playlist_prev_song: 'p',
            volume_up: '+',
            volume_down: '-',
        }
    }
}

impl KeyConfig {}
