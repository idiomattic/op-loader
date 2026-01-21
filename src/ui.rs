use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::{Account, App, FocusedPanel, Vault, VaultItem};

pub fn render(frame: &mut Frame, app: &mut App) {
    let outer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    let left_pane_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(outer_layout[0]);

    let right_pane_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(outer_layout[1]);

    render_list_panel(&AccountListPanel, frame, app, left_pane_layout[0]);
    render_list_panel(&VaultListPanel, frame, app, left_pane_layout[1]);
    render_list_panel(&VaultItemListPanel, frame, app, right_pane_layout[0]);
}

trait ListPanel {
    type Item;

    fn title(&self) -> &str;
    fn focus_variant(&self) -> FocusedPanel;
    fn selected_color(&self) -> Color;

    fn items<'a>(&self, app: &'a App) -> &'a [Self::Item];

    fn display_item(&self, item: &Self::Item) -> String;

    fn selected_idx(&self, app: &App) -> Option<usize>;
    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState;
}

fn render_list_panel<P: ListPanel>(panel: &P, frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = &app.focused_panel == &panel.focus_variant();
    let selected_idx = panel.selected_idx(app);
    let selected_color = panel.selected_color();

    let block = Block::default()
        .title(panel.title())
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let items: Vec<ListItem> = panel
        .items(app)
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = selected_idx == Some(idx);
            let prefix = if is_selected { "● " } else { "  " };
            let content = format!("{}{}", prefix, panel.display_item(item));

            ListItem::new(content).style(if is_selected {
                Style::default().fg(selected_color)
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

    frame.render_stateful_widget(list, area, panel.list_state(app));
}

struct AccountListPanel;

impl ListPanel for AccountListPanel {
    type Item = Account;

    fn title(&self) -> &str {
        " [0] Accounts "
    }
    fn focus_variant(&self) -> FocusedPanel {
        FocusedPanel::AccountList
    }
    fn items<'a>(&self, app: &'a App) -> &'a [Account] {
        &app.accounts
    }
    fn display_item(&self, item: &Self::Item) -> String {
        item.email.clone()
    }
    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.account_list_state
    }
    fn selected_color(&self) -> Color {
        Color::Cyan
    }
    fn selected_idx(&self, app: &App) -> Option<usize> {
        app.selected_account_idx
    }
}

struct VaultListPanel;

impl ListPanel for VaultListPanel {
    type Item = Vault;

    fn title(&self) -> &str {
        " [1] Vaults "
    }
    fn focus_variant(&self) -> FocusedPanel {
        FocusedPanel::VaultList
    }
    fn items<'a>(&self, app: &'a App) -> &'a [Vault] {
        &app.vaults
    }
    fn display_item(&self, item: &Self::Item) -> String {
        item.name.clone()
    }
    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.vault_list_state
    }
    fn selected_color(&self) -> Color {
        Color::Cyan
    }
    fn selected_idx(&self, app: &App) -> Option<usize> {
        app.selected_vault_idx
    }
}

struct VaultItemListPanel;
impl ListPanel for VaultItemListPanel {
    type Item = VaultItem;

    fn title(&self) -> &str {
        " [2] Items "
    }
    fn focus_variant(&self) -> FocusedPanel {
        FocusedPanel::VaultItemList
    }
    fn items<'a>(&self, app: &'a App) -> &'a [VaultItem] {
        &app.vault_items
    }
    fn display_item(&self, item: &Self::Item) -> String {
        item.title.clone()
    }
    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.vault_item_list_state
    }
    fn selected_color(&self) -> Color {
        Color::Cyan
    }
    fn selected_idx(&self, app: &App) -> Option<usize> {
        app.selected_vault_item_idx
    }
}
