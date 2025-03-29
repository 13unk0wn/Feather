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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Navigation {
    home: char,
    quit: char,
    search: char,
    player: char,
    history: char,
    userplaylist: char,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PlaylistKeyBindings {
    switch_mode: char, // Switch Mode b/w ViewPlaylistSearch and SearchPlaylist

    playlist_search: PlayListSearchKeyBindings,
    view_playlist: PlayListViewKeyBindings,
}
// USERPLAYLIST WILL ALSO FOLLOW THE SAME KEYBINDINGS
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PlayListSearchKeyBindings {
    switch_mode: char, // it switch from playlist_search_box to playlist_view_box
    select_playlist: Option<char>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PlayListViewKeyBindings {
    start_playlist: char,  // these key is used start playlist from song one
    start_from_here: char, // these key is used start song from the selected song
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SongSearchKeyBinding {
    switch_mode: char, // switch from song textbox to song search list
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

impl KeyConfig {}
