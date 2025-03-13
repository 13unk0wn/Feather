use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Row, Table, Widget},
};

// Currently these key-bindings are not valid
pub struct Help;

impl Help {
    pub fn new() -> Help {
        Help {}
    }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let rows = vec![
            Row::new(vec![Cell::from("s"), Cell::from("Search")]),
            Row::new(vec![Cell::from("h"), Cell::from("History")]),
            Row::new(vec![Cell::from("p"), Cell::from("Player")]),
            Row::new(vec![Cell::from("?"), Cell::from("Toggle Help Mode")]),
            Row::new(vec![
                Cell::from("TAB (Search)"),
                Cell::from("Toggle between search input and results"),
            ]),
            Row::new(vec![
                Cell::from("Esc (Global)"),
                Cell::from("Quit application"),
            ]),
            Row::new(vec![
                Cell::from("Esc (Non-Global)"),
                Cell::from("Switch to Global Mode"),
            ]),
            Row::new(vec![
                Cell::from("↑ / k(History/Search)"),
                Cell::from("Navigate up in list"),
            ]),
            Row::new(vec![
                Cell::from("↓ / j(History/Search)"),
                Cell::from("Navigate down in list"),
            ]),
            Row::new(vec![
                Cell::from("Space / ; (Player)"),
                Cell::from("Pause current song"),
            ]),
            Row::new(vec![
                Cell::from("→ (Player)"),
                Cell::from("Skip forward 5 seconds"),
            ]),
            Row::new(vec![
                Cell::from("← (Player)"),
                Cell::from("Rewind 5 seconds"),
            ]),
        ];

        let help_table = Table::new(
            rows,
            [Constraint::Percentage(20), Constraint::Percentage(80)],
        )
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .header(Row::new(vec![Cell::from("Key"), Cell::from("Action")]));

        help_table.render(area, buf);
    }
}
