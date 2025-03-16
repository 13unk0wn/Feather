#![allow(unused)]
use rascii_art::{render_to, RenderOptions};
use serde_json::Error;
use std::path::PathBuf;
use thiserror::Error;

use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;

use ratatui::prelude::Buffer;

#[derive(Error, Debug)]
enum HomeErorr {
    #[error("Image not Found : {0}")]
    ImageNotFound(String),
}

pub struct Home {
    image_path: Option<String>,
}

impl Home {
    pub fn new() -> Self {
        Self { image_path: None }
    }

    fn convert_image_text_and_save(&mut self, image_dir: PathBuf) -> Result<(), HomeErorr> {
        let mut image_string = String::new();
        if !image_dir.is_file() {
            return Err(HomeErorr::ImageNotFound(format!(
                "{:?} not found",
                image_dir
            )));
        }
        let image_dir_str = image_dir.to_str().unwrap();
        let _ = render_to(
            &image_dir_str,
            &mut image_string,
            &RenderOptions::new().width(40).height(20).colored(true),
        )
        .expect("Ascii image Conversion failed");
        Ok(())
    }

    fn write_string_to_file(&self, image_str: &str) -> Result<(), HomeErorr> {
        Ok(())
    }

    pub fn handle_keywords(&self, key: KeyEvent) {}

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {}
}
