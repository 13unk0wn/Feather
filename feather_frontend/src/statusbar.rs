#![allow(unused)]
use crate::State;
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

pub struct StatusBar {
    state: State,
}

impl StatusBar {
    pub fn new() -> Self {
        Self { state: State::Home }
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

        match self.state {
            State::Home => {
                let keystroke_bar = Line::from(vec![
                    Span::styled("[:q→Quit] ", Style::default().fg(Color::Yellow)),
                    Span::styled("[:s→Search] ", Style::default().fg(Color::Yellow)),
                    Span::styled("[:h→History] ", Style::default().fg(Color::Yellow)),
                    Span::styled("[:u→UserPlayList] ", Style::default().fg(Color::Yellow)),
                    Span::styled("[h/j/k/l→ Scroll] ", Style::default().fg(Color::Yellow)),
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
