#![allow(unused)]
use crate::State;
use color_eyre::owo_colors::OwoColorize;
use feather::config::KeyConfig;
use feather::config::USERCONFIG;
use ratatui::prelude::Buffer;
use ratatui::prelude::Constraint;
use ratatui::prelude::Direction;
use ratatui::prelude::Layout;
use ratatui::prelude::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use std::rc::Rc;

pub struct StatusBar {
    state: State,
    config: Rc<USERCONFIG>,
    key_config: Rc<KeyConfig>,
}

impl StatusBar {
    pub fn new(config: Rc<USERCONFIG>, key_config: Rc<KeyConfig>) -> Self {
        Self {
            state: State::Home,
            config,
            key_config,
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer, state: State) {
        self.state = state;
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20), // Space for the keystroke bar
                Constraint::Percentage(60), // Empty space at the top
                Constraint::Percentage(10), // Space for the keystroke bar
            ])
            .split(area);
        let status_block = Block::default().borders(Borders::TOP);

        let color = self.config.selected_tab_color;
        match self.state {
            State::Home => {
                let leader = &self.key_config.leader;
                let search = &self.key_config.navigation.search;
                let player = &self.key_config.navigation.player;
                let history = &self.key_config.navigation.history;
                let userplaylist = &self.key_config.navigation.userplaylist;

                let keystroke_bar = Line::from(vec![
                    Span::styled(
                        format!("[{}{}→Search] ", leader, search),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}{}→Player] ", leader, player),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}{}→History] ", leader, history),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}{}→UserPlaylist]", leader, userplaylist),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                ]);
                status_block
                    .title(keystroke_bar)
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .render(vertical_layout[1], buf);
            }
            State::History => {
                let home = self.key_config.navigation.home;
                let leader = self.key_config.leader;
                let add_to_playlist = self
                    .key_config
                    .history
                    .add_to_playlist
                    .unwrap_or(self.key_config.default.add_to_playlist);
                let up = self
                    .key_config
                    .history
                    .up
                    .unwrap_or(self.key_config.default.up);
                let down = self
                    .key_config
                    .history
                    .down
                    .unwrap_or(self.key_config.default.down);
                let play_song = self
                    .key_config
                    .history
                    .play_song
                    .unwrap_or(self.key_config.default.play_song);

                let delete = self.key_config.history.delete;
                let keystroke_bar = Line::from(vec![
                    Span::styled(
                        format!("[{}{}→Home] ", leader, home),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}→add_to_playlist] ", add_to_playlist),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[({}/▲)/({}/▼)→Navigation] ", up, down),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}/ENTER→play_song] ", play_song),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}→delete_song]", delete),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                ]);
                status_block
                    .title(keystroke_bar)
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .render(vertical_layout[1], buf);
            }
            State::SongPlayer => {
                let pause_song = self.key_config.player.pause;
                let skip_plus_secs = self.key_config.player.skip_plus_secs;
                let skip_minus_secs = self.key_config.player.skip_minus_secs;
                let playlist_next_song = self.key_config.player.playlist_next_song;
                let playlist_prev_song = self.key_config.player.playlist_prev_song;
                let volume_up = self.key_config.player.volume_up;
                let volume_down = self.key_config.player.volume_down;
                let keystroke_bar = Line::from(vec![
                    Span::styled(
                        format!("[{}/SPACE→pause_song] ", pause_song),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[({}/→)→Skip+] ", skip_plus_secs),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[({}/←)→Skip-] ", skip_minus_secs),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}→playlist_next_song] ", playlist_next_song),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[{}→playlist_prev_song] ", playlist_prev_song),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[({}/↑)→volume_up] ", volume_up),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                    Span::styled(
                        format!("[({}/↓)→volume_down]", volume_down),
                        Style::default().fg(Color::Rgb(color.0, color.1, color.2)),
                    ),
                ]);
                status_block
                    .title(keystroke_bar)
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .render(vertical_layout[1], buf);
            }
            State::Search => {
                // let keystroke_bar = Line::from(vec![
                //     Span::styled("[;→Switch Mode] ", Style::default().fg(Color::Yellow)),
                //     Span::styled("[:→] ", Style::default().fg(Color::Yellow)),
                //     Span::styled("[:h→History] ", Style::default().fg(Color::Yellow)),
                //     Span::styled("[:u→UserPlayList] ", Style::default().fg(Color::Yellow)),
                //     Span::styled("[h/j/k/l→ Scroll] ", Style::default().fg(Color::Yellow)),
                // ]);
                // status_block
                //     .title(keystroke_bar)
                //     .title_alignment(ratatui::layout::Alignment::Center)
                //     .render(vertical_layout[1], buf);
            }
            _ => (),
        }
    }
}
