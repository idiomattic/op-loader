use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let outer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    let left_pane_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(outer_layout[0]);

    render_vault_list(frame, app, left_pane_layout[1]);
}

fn render_vault_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = true;

    let block = Block::default()
        .title(" Vaults ")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = app
        .vaults
        .iter()
        .enumerate()
        .map(|(idx, vault)| {
            let is_selected = app.selected_vault_idx == Some(idx);
            let prefix = if is_selected { "● " } else { "  " };
            let content = format!("{}{}", prefix, vault.name);

            ListItem::new(content).style(if is_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            })
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.vault_list_state);
}
