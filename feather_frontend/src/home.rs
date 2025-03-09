#![allow(unused)]
use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;

use ratatui::prelude::Buffer;

pub struct Home {
    image_path: Option<PathBuf>,
}

impl Home {
    pub fn new() -> Self {
        Self { image_path: None }
    }

    fn convert_image_text_and_save(&mut self, image_dir: PathBuf) {}

    pub fn handle_keywords(&self, key: KeyEvent) {}

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {}
}
