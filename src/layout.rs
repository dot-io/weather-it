use ratatui::layout::{Constraint, Flex, Layout, Rect};

pub fn center(area: Rect, width: u16) -> Rect {
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    area
}
