#![allow(unused)]
use crate::backend::{self, Backend};
use color_eyre::owo_colors::OwoColorize;
use feather::database::FAVOURITE_SONGS_SIZE;
use log::debug;
use log::log;
use ratatui::widgets::List;
use ratatui::widgets::ListState;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;

use ratatui::prelude::Constraint;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use feather::config::USERCONFIG;
use ratatui::prelude::Widget;
use ratatui::text::Text;
use ratatui::widgets::ListItem;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
enum HomeErorr {
    #[error("Image not Found : {0}")]
    ImageNotFound(String),
}

pub struct Home {
    backend: Arc<Backend>,
    config: Rc<USERCONFIG>,
    favourite_songs: FavoriteSongs,
}

impl Home {
    pub fn new(backend: Arc<Backend>, config: Rc<USERCONFIG>) -> Self {
        let user = Self {
            backend: backend.clone(),
            config: config.clone(),
            favourite_songs: FavoriteSongs::new(backend, config),
        };

        user.backend
            .user_profile
            .check_pfp_change(user.config.clone())
            .unwrap();

        user
    }

    pub fn handle_keywords(&mut self, key: KeyEvent) {
        self.favourite_songs.handle_keystrokes(key);
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let fixed_width = 80;
        let fixed_height = 40;
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(fixed_width), Constraint::Min(0)])
            .split(area);

        let image_area = chunks[0];
        let image_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(fixed_height)])
            .split(image_area)[0];
        let stats_area = chunks[1];

        let stats_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(stats_area);

        let get_data = self.backend.user_profile.give_info().unwrap();

        let ascii_art_lines: Vec<Line> = get_data
            .pfp
            .split('\n') // Split manually
            .map(|line| Line::from(Span::raw(line.to_string()))) // Convert to `Line`
            .collect();

        // Manually create a `Text` object instead of directly using `Paragraph`
        let ascii_text = Text::from(ascii_art_lines);

        let image_block = Block::default().borders(Borders::ALL);

        let selected_tab_color =
            (self.config.image_color).unwrap_or(self.config.selected_tab_color);
        // Create `Paragraph` with explicit `Text`
        let image_paragraph = Paragraph::new(ascii_text)
            .block(image_block)
            .style(Style::default().fg(Color::Rgb(
                selected_tab_color.0,
                selected_tab_color.1,
                selected_tab_color.2,
            )))
            .alignment(Alignment::Left);
        image_paragraph.render(image_area, buf);

        let user_stats = vec![
            Line::from(vec![
                Span::styled(
                    "👤 User: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(get_data.name, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "🎵 Last Played: ",
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    get_data
                        .last_played
                        .as_ref()
                        .map(|s| s.title.clone())
                        .unwrap_or("None".to_string()),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "📀 Songs Played: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    get_data.songs_played.to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "⏳ Time Played: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} mins", get_data.time_played / 60),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        let stats_block = Block::default()
            .borders(Borders::ALL)
            .title(" 🎼 USER STATS ")
            .title_alignment(Alignment::Center);

        let paragraph = Paragraph::new(user_stats)
            .block(stats_block)
            .alignment(Alignment::Left);

        paragraph.render(stats_chunks[0], buf);
        self.favourite_songs.render(stats_chunks[1], buf);
    }
}

struct FavoriteSongs {
    backend: Arc<Backend>,
    config: Rc<USERCONFIG>,
    selected: usize,
    max_len: usize,
    vertical_scroll_state: ScrollbarState,
}

impl FavoriteSongs {
    fn new(backend: Arc<Backend>, config: Rc<USERCONFIG>) -> Self {
        Self {
            backend,
            config,
            selected: 0,
            max_len: FAVOURITE_SONGS_SIZE,
            vertical_scroll_state: ScrollbarState::default(),
        }
    }

    pub fn handle_keystrokes(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                // Move selection down
                self.select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Move selection up
                self.select_previous();
            }
            _ => (),
        }
    }

    // Moves selection to next item, respecting bounds
    fn select_next(&mut self) {
        if self.max_len > 0 {
            self.selected = (self.selected + 1).min(self.max_len - 1);
            self.vertical_scroll_state = self.vertical_scroll_state.position(self.selected);
        }
    }

    // Moves selection to previous item, preventing underflow
    fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.selected);
    }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let selected_item_text_color = self.config.selected_list_item;
        let selected_item_bg = self.config.selected_tab_color;

        let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        scrollbar.render(area, buf, &mut self.vertical_scroll_state);
        if let Ok(items) = self.backend.history.most_played() {
            self.max_len = items.len();
            let view_items: Vec<ListItem> = items
                .into_iter()
                .enumerate()
                .map(|(i, item)| {
                    // Format each item for display
                    let is_selected = i == self.selected;

                    let style = if is_selected {
                        // Highlight selected item
                        Style::default()
                            .fg(Color::Rgb(
                                selected_item_text_color.0,
                                selected_item_text_color.1,
                                selected_item_text_color.0,
                            ))
                            .bg(Color::Rgb(
                                selected_item_bg.0,
                                selected_item_bg.1,
                                selected_item_bg.2,
                            ))
                    } else {
                        Style::default()
                    };
                    let text = format!("{} - {}", item.song_name, item.artist_name.join(", "));
                    ListItem::new(Span::styled(text, style))
                })
                .collect();
            let mut list_state = ListState::default();
            list_state.select(Some(self.selected));

            StatefulWidget::render(
                // Render the list
                List::new(view_items)
                    .block(
                        Block::default()
                            .title("Favourites")
                            .title_alignment(Alignment::Center)
                            .borders(Borders::ALL),
                    )
                    .highlight_symbol(&self.config.selected_item_char),
                area,
                buf,
                &mut list_state,
            );
        }
    }
}
