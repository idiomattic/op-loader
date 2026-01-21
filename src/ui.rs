use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::{App, FocusedPanel};

pub fn render(frame: &mut Frame, app: &mut App) {
    let outer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    let left_pane_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(outer_layout[0]);

    render_account_list(frame, app, left_pane_layout[0]);
    render_vault_list(frame, app, left_pane_layout[1]);
}

fn render_account_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = matches!(app.focused_panel, FocusedPanel::AccountList);

    let block = Block::default()
        .title(" [0] Accounts ")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = app
        .accounts
        .iter()
        .enumerate()
        .map(|(idx, account)| {
            let is_selected = app.selected_account_idx == Some(idx);
            let prefix = if is_selected { "● " } else { "  " };
            let content = format!("{}{}", prefix, account.email);

            ListItem::new(content).style(if is_selected {
                Style::default().fg(Color::Cyan)
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

    frame.render_stateful_widget(list, area, &mut app.account_list_state);
}

fn render_vault_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = matches!(app.focused_panel, FocusedPanel::VaultList);

    let block = Block::default()
        .title(" [1] Vaults ")
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
