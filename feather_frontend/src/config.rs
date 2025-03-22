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
