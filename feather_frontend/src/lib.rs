pub mod backend;
pub mod config;
pub mod delete_userplaylist;
pub mod help;
pub mod history;
pub mod home;
pub mod player;
pub mod playlist_search;
pub mod popup_playlist;
pub mod search;
pub mod search_main;
pub mod statusbar;
pub mod userplaylist;
pub mod error;

/// Enum representing different states of the application.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum State {
    Home,
    HelpMode,
    Search,
    History,
    UserPlaylist,
    // CurrentPlayingPlaylist,
    SongPlayer,
}
