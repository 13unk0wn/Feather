#![allow(unused)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;
#[derive(Serialize, Deserialize, Debug)]
pub struct USERCONFIG {
    pub bg_color: (u8, u8, u8),
    pub text_color: (u8, u8, u8),
    pub play_icon: String,
    pub pause_icon: String,
    pub selected_list_item: (u8, u8, u8),
    pub selected_item_char: char,
    pub selected_tab_color: (u8, u8, u8),
    pub player_progress_bar_color: (u8, u8, u8),
    pub player_volume_bar_color: (u8, u8, u8),
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
            bg_color: (0, 0, 0),         // Black
            text_color: (255, 255, 255), // White
            play_icon: "▶".to_string(),
            pause_icon: "❚❚".to_string(),
            selected_list_item: (50, 50, 50), // Dark gray
            selected_item_char: '>',
            selected_tab_color: (100, 100, 255),    // Blueish
            player_progress_bar_color: (200, 0, 0), // Red
            player_volume_bar_color: (0, 200, 0),   // Green
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
