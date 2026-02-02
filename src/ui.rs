use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{Account, App, FocusedPanel, ItemField, Vault};
use crate::command_log::CommandLogEntry;

pub fn render(frame: &mut Frame, app: &mut App) {
    let outer_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    let left_pane_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .split(outer_layout[0]);

    let right_pane_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1)])
        .split(outer_layout[1]);

    render_list_panel(&AccountListPanel, frame, app, left_pane_layout[0]);
    render_list_panel(&VaultListPanel, frame, app, left_pane_layout[1]);
    render_command_log(frame, app, left_pane_layout[2]);
    render_vault_item_panel(frame, app, right_pane_layout[0]);

    if app.modal_open {
        render_modal(frame, app);
    }
}

trait ListPanel {
    type Item;

    fn title(&self) -> &str;
    fn title_bottom(&self) -> Option<&str> {
        None
    }
    fn focus_variant(&self) -> FocusedPanel;
    fn selected_color(&self) -> Color;

    fn items<'a>(&self, app: &'a App) -> &'a [Self::Item];

    fn display_item(&self, item: &Self::Item) -> String;

    fn is_favorite(&self, _app: &App, _item: &Self::Item) -> bool {
        false
    }

    fn selected_idx(&self, app: &App) -> Option<usize>;
    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState;
}

fn render_list_panel<P: ListPanel>(panel: &P, frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_panel == panel.focus_variant();

    let mut block = Block::default()
        .title(panel.title())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    if let Some(title_bottom) = panel.title_bottom() {
        block = block.title_bottom(Line::from(title_bottom).right_aligned());
    }

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    render_list_inner(panel, frame, app, inner_area);
}

fn render_list_inner<P: ListPanel>(panel: &P, frame: &mut Frame, app: &mut App, area: Rect) {
    let selected_idx = panel.selected_idx(app);
    let selected_color = panel.selected_color();

    let items: Vec<ListItem> = panel
        .items(app)
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = selected_idx == Some(idx);
            let is_favorite = panel.is_favorite(app, item);
            let prefix = if is_selected { "● " } else { "  " };
            let suffix = if is_favorite { " ★" } else { "" };
            let content = format!("{}{}{}", prefix, panel.display_item(item), suffix);

            ListItem::new(content).style(if is_selected {
                Style::default().fg(selected_color)
            } else {
                Style::default()
            })
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, panel.list_state(app));
}

fn render_vault_item_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_panel == FocusedPanel::VaultItemList && !app.search_active;

    let block = Block::default()
        .title(" [2] Items ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Percentage(50),
        ])
        .split(inner);

    render_filtered_vault_items(frame, app, chunks[0]);
    render_search_box(frame, app, chunks[1]);
    render_item_details(frame, app, chunks[2]);
}

fn render_filtered_vault_items(frame: &mut Frame, app: &mut App, area: Rect) {
    let selected_idx = app.selected_vault_item_idx;

    let items: Vec<ListItem> = app
        .filtered_item_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &real_idx)| {
            let item = &app.vault_items[real_idx];
            let is_selected = selected_idx == Some(display_idx);
            let prefix = if is_selected { "● " } else { "  " };
            let content = format!("{}{}", prefix, item.title);

            ListItem::new(content).style(if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            })
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut app.vault_item_list_state);
}

fn render_search_box(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.search_active;

    let block = Block::default()
        .title(" [/] Search ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = if app.search_query.is_empty() {
        if is_active {
            String::new()
        } else {
            "Press / to search".to_string()
        }
    } else if is_active {
        format!("{}█", app.search_query)
    } else {
        app.search_query.clone()
    };

    let style = if app.search_query.is_empty() && !is_active {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };

    let paragraph = Paragraph::new(text).style(style);
    frame.render_widget(paragraph, inner);
}

fn render_item_details(frame: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = app.focused_panel == FocusedPanel::VaultItemDetail;

    let block = Block::default()
        .title(" [3] Details ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(details) = &app.selected_item_details else {
        let empty = Paragraph::new("Select an item and press Enter");
        frame.render_widget(empty, inner);
        return;
    };

    let fields: Vec<&ItemField> = details
        .fields
        .iter()
        .filter(|f| f.label != "notesPlain")
        .collect();

    let items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(idx, f)| {
            let is_selected = app.selected_field_idx == Some(idx);
            let value = if f.field_type == "CONCEALED" {
                "********".to_string()
            } else {
                f.value.clone().unwrap_or_default()
            };
            let prefix = if is_selected { "● " } else { "  " };
            let content = format!("{}{}: {}\n    {}", prefix, f.label, value, f.reference);

            ListItem::new(content).style(if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            })
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, inner, &mut app.item_detail_list_state);
}

fn render_command_log(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Command Log ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let visible_lines = area.height.saturating_sub(2) as usize;

    let text: String = app
        .command_log
        .recent(visible_lines)
        .iter()
        .map(CommandLogEntry::display)
        .collect::<Vec<_>>()
        .join("\n");

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_modal(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Content: field info (5) + spacer (1) + input (3) + error (1) + help (1) = 11, plus border (2) = 13
    let modal_width = area.width * 60 / 100;
    let modal_height = 13_u16.min(area.height - 4);
    let modal_x = (area.width - modal_width) / 2;
    let modal_y = (area.height - modal_height) / 2;

    let modal_area = Rect::new(modal_x, modal_y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Save to Configuration ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // field info
            Constraint::Length(1), // spacer
            Constraint::Length(3), // env var input
            Constraint::Length(1), // error message
            Constraint::Length(1), // help text
        ])
        .split(inner);

    if let Some(field) = app.modal_selected_field() {
        let value_display = if field.field_type == "CONCEALED" {
            "********".to_string()
        } else {
            field.value.clone().unwrap_or_default()
        };

        let info_text = format!(
            "Field: {}\nValue: {}\n\nReference:\n{}",
            field.label, value_display, field.reference
        );

        let info = Paragraph::new(info_text).wrap(Wrap { trim: false });
        frame.render_widget(info, chunks[0]);
    }

    let input_block = Block::default()
        .title(" Environment Variable Name ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let input_inner = input_block.inner(chunks[2]);
    frame.render_widget(input_block, chunks[2]);

    let input_text = format!("{}█", app.modal_env_var_name);
    let input = Paragraph::new(input_text);
    frame.render_widget(input, input_inner);

    if let Some(ref error) = app.error_message {
        let error_text = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        frame.render_widget(error_text, chunks[3]);
    }

    let help = Paragraph::new("Enter: Save  |  Esc: Cancel")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, chunks[4]);
}

struct AccountListPanel;

impl ListPanel for AccountListPanel {
    type Item = Account;

    fn title(&self) -> &'static str {
        " [0] Accounts "
    }
    fn title_bottom(&self) -> Option<&str> {
        Some(" [f] Favorite ")
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
    fn is_favorite(&self, app: &App, item: &Self::Item) -> bool {
        app.config
            .as_ref()
            .and_then(|c| c.default_account_id.as_ref())
            .is_some_and(|id| id == &item.account_uuid)
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

    fn title(&self) -> &'static str {
        " [1] Vaults "
    }
    fn title_bottom(&self) -> Option<&str> {
        Some(" [f] Favorite ")
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
    fn is_favorite(&self, app: &App, item: &Self::Item) -> bool {
        app.selected_account()
            .map(|a| a.account_uuid.clone())
            .and_then(|account_id| {
                app.config
                    .as_ref()
                    .and_then(|c| c.default_vault_per_account.get(&account_id))
            })
            .is_some_and(|vault_id| vault_id == &item.id)
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
