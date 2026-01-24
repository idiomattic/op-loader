use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::ListState;

use crate::app::{App, FocusedPanel};

enum NavAction {
    Up,
    Down,
    Select,
    Quit,
    PanelZero,
    PanelOne,
    PanelTwo,
    PanelFour,
}

impl NavAction {
    fn from_key(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(Self::Up),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(Self::Down),
            KeyCode::Enter => Some(Self::Select),
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(Self::Quit),
            KeyCode::Char('0') => Some(Self::PanelZero),
            KeyCode::Char('1') => Some(Self::PanelOne),
            KeyCode::Char('2') => Some(Self::PanelTwo),
            KeyCode::Char('3') => Some(Self::PanelFour),
            _ => None,
        }
    }
}

pub fn handle_events(app: &mut App) -> Result<()> {
    if let Event::Key(key) = event::read().context("Failed to read keyboard event")?
        && key.kind == KeyEventKind::Press
    {
        handle_key_press(app, key);
    }
    Ok(())
}

fn handle_key_press(app: &mut App, key: KeyEvent) {
    if app.modal_open {
        match key.code {
            KeyCode::Esc => {
                app.close_modal();
            }
            KeyCode::Enter => {
                if app.modal_env_var_name.is_empty() {
                    app.error_message =
                        Some("Environment variable name cannot be empty".to_string());
                    return;
                }

                if let Some(ref op_reference) = app.modal_field_reference.clone() {
                    match app.save_op_item_config(&app.modal_env_var_name.clone(), op_reference) {
                        Ok(()) => {
                            app.command_log.log_success(
                                format!("Saved {} to config", app.modal_env_var_name),
                                None,
                            );
                            app.close_modal();
                        }
                        Err(e) => app.error_message = Some(e.to_string()),
                    }
                }
            }
            KeyCode::Backspace => {
                app.modal_env_var_name.pop();
                app.error_message = None;
            }
            KeyCode::Char(c) => {
                if c.is_ascii_alphanumeric() || c == '_' {
                    app.modal_env_var_name.push(c.to_ascii_uppercase());
                    app.error_message = None;
                }
            }
            _ => {}
        }
        return;
    }

    if app.search_active {
        match key.code {
            KeyCode::Esc => {
                app.clear_search();
            }
            KeyCode::Enter => {
                app.search_active = false;
                VaultItemListNav.on_select(app);
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.update_filtered_items();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.update_filtered_items();
            }
            KeyCode::Up => VaultItemListNav.handle_up(app),
            KeyCode::Down => VaultItemListNav.handle_down(app),
            _ => {}
        }
        return;
    }

    if key.code == KeyCode::Char('/')
        && (app.focused_panel == FocusedPanel::VaultItemList
            || app.focused_panel == FocusedPanel::VaultItemDetail)
    {
        app.search_active = true;
        return;
    }

    // TODO: use `fn ensure_handle_action()` pattern?
    if key.code == KeyCode::Char('f') || key.code == KeyCode::Char('F') {
        match app.focused_panel {
            FocusedPanel::AccountList => {
                if let Some(selected_account_id) = app
                    .account_list_state
                    .selected()
                    .and_then(|idx| app.accounts.get(idx))
                    .map(|a| a.account_uuid.clone())
                {
                    match app.set_default_account(&selected_account_id) {
                        Err(e) => {
                            app.command_log.log_failure(
                                "Failed to save default account configuration",
                                e.to_string(),
                            );
                        }
                        Ok(()) => {
                            app.command_log
                                .log_success("Saved default account configuration", None);
                            AccountListNav.on_select(app);
                        }
                    }
                }
            }
            FocusedPanel::VaultList => {
                if let Some(selected_vault_id) = app
                    .vault_list_state
                    .selected()
                    .and_then(|idx| app.vaults.get(idx))
                    .map(|v| v.id.clone())
                {
                    match app.set_default_vault(&selected_vault_id) {
                        Err(e) => {
                            app.command_log.log_failure(
                                "Failed to save default vault configuration",
                                e.to_string(),
                            );
                        }
                        Ok(()) => {
                            app.command_log
                                .log_success("Saved default vault configuration", None);
                            VaultListNav.on_select(app);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(action) = NavAction::from_key(key.code) {
        match action {
            NavAction::Quit => app.should_quit = true,
            NavAction::PanelZero => app.focused_panel = FocusedPanel::AccountList,
            NavAction::PanelOne => app.focused_panel = FocusedPanel::VaultList,
            NavAction::PanelTwo => app.focused_panel = FocusedPanel::VaultItemList,
            NavAction::PanelFour => app.focused_panel = FocusedPanel::VaultItemDetail,
            nav_action => {
                let nav: &dyn ListNav = match app.focused_panel {
                    FocusedPanel::AccountList => &AccountListNav,
                    FocusedPanel::VaultList => &VaultListNav,
                    FocusedPanel::VaultItemList => &VaultItemListNav,
                    FocusedPanel::VaultItemDetail => &VaultItemDetailNav,
                };

                match nav_action {
                    NavAction::Up => nav.handle_up(app),
                    NavAction::Down => nav.handle_down(app),
                    NavAction::Select => nav.on_select(app),
                    _ => unreachable!(),
                }
            }
        }
    }
}

trait ListNav {
    fn len(&self, app: &App) -> usize;

    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState;

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>);

    fn handle_up(&self, app: &mut App) {
        let len = self.len(app);
        if len == 0 {
            return;
        }

        let state = self.list_state(app);
        let idx = state.selected().unwrap_or(0);
        let new_idx = if idx == 0 { len - 1 } else { idx - 1 };
        state.select(Some(new_idx));
    }
    fn handle_down(&self, app: &mut App) {
        let len = self.len(app);
        if len == 0 {
            return;
        }

        let state = self.list_state(app);
        let idx = state.selected().unwrap_or(0);
        let new_idx = if idx == len - 1 { 0 } else { idx + 1 };
        state.select(Some(new_idx))
    }
    fn on_select(&self, app: &mut App) {
        let idx = self.list_state(app).selected();
        self.set_selected_idx(app, idx);
    }
}

struct AccountListNav;
impl ListNav for AccountListNav {
    fn len(&self, app: &App) -> usize {
        app.accounts.len()
    }

    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.account_list_state
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.selected_account_idx = idx;
    }
}

struct VaultListNav;
impl ListNav for VaultListNav {
    fn len(&self, app: &App) -> usize {
        app.vaults.len()
    }

    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.vault_list_state
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.selected_vault_idx = idx;
    }

    fn on_select(&self, app: &mut App) {
        let idx = self.list_state(app).selected();
        self.set_selected_idx(app, idx);

        app.clear_search();

        if let Err(e) = app.load_vault_items() {
            app.error_message = Some(e.to_string());
        }

        app.focused_panel = FocusedPanel::VaultItemList;
    }
}

struct VaultItemListNav;
impl ListNav for VaultItemListNav {
    fn len(&self, app: &App) -> usize {
        app.filtered_item_indices.len()
    }

    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.vault_item_list_state
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.selected_vault_item_idx = idx;
    }

    fn on_select(&self, app: &mut App) {
        let list_idx = self.list_state(app).selected();
        self.set_selected_idx(app, list_idx);

        if let Some(list_idx) = list_idx
            && let Some(&real_idx) = app.filtered_item_indices.get(list_idx)
            && let Some(item) = app.vault_items.get(real_idx)
        {
            let item_id = item.id.clone();
            if let Err(e) = app.load_item_details(&item_id) {
                app.error_message = Some(e.to_string());
            } else {
                app.item_detail_list_state.select(Some(0));
                app.selected_field_idx = None;
                app.focused_panel = FocusedPanel::VaultItemDetail;
            }
        }
    }
}

struct VaultItemDetailNav;
impl ListNav for VaultItemDetailNav {
    fn len(&self, app: &App) -> usize {
        app.selected_item_details
            .as_ref()
            .map(|d| d.fields.iter().filter(|f| f.label != "notesPlain").count())
            .unwrap_or(0)
    }

    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.item_detail_list_state
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.selected_field_idx = idx;
    }

    fn on_select(&self, app: &mut App) {
        let list_idx = self.list_state(app).selected();
        self.set_selected_idx(app, list_idx);

        if let Some(idx) = list_idx
            && let Some(details) = &app.selected_item_details
        {
            let field = details
                .fields
                .iter()
                .filter(|f| f.label != "notesPlain")
                .nth(idx);

            if let Some(field) = field {
                app.open_modal(field.reference.clone());
            }
        }
    }
}
