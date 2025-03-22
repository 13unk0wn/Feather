use ratatui::widgets::Clear;
use ratatui::{
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::config::USERCONFIG;

pub struct ErrorPopUp {
    error_message: Arc<Mutex<Option<String>>>,
    error_timer: Arc<Mutex<Option<Instant>>>,
    config: Arc<USERCONFIG>,
}

impl ErrorPopUp {
    pub fn new(config: Arc<USERCONFIG>) -> Self {
        Self {
            error_message: Arc::new(Mutex::new(None)),
            error_timer: Arc::new(Mutex::new(None)),
            config,
        }
    }

    pub fn show_error(&self, msg: String) {
        let error_message = Arc::clone(&self.error_message);
        let error_timer = Arc::clone(&self.error_timer);

        // Update the error message and start the timer
        *error_message.lock().unwrap() = Some(msg);
        *error_timer.lock().unwrap() = Some(Instant::now());

        // Hide after 3 seconds
        let error_message = Arc::clone(&self.error_message);
        let error_timer = Arc::clone(&self.error_timer);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            *error_message.lock().unwrap() = None;
            *error_timer.lock().unwrap() = None;
        });
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if let Some(msg) = self.error_message.lock().unwrap().as_ref() {
            Clear.render(area, buf);
            let bg_color = self.config.bg_color;
            let text_color = self.config.text_color;
            let global_style = Style::default()
                .fg(Color::Rgb(text_color.0, text_color.1, text_color.2))
                .bg(Color::Rgb(bg_color.0, bg_color.1, bg_color.2));

            Block::default().style(global_style).render(area, buf);
            let block = Block::default()
                .title("Error")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Red));

            let paragraph = Paragraph::new(msg.clone())
                .block(block)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);

            paragraph.render(area, buf);
        }
    }
}
