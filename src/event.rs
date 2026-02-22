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
    PanelVars,
}

impl NavAction {
    const fn from_key(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Up | KeyCode::Char('k' | 'K') => Some(Self::Up),
            KeyCode::Down | KeyCode::Char('j' | 'J') => Some(Self::Down),
            KeyCode::Enter => Some(Self::Select),
            KeyCode::Char('q' | 'Q') => Some(Self::Quit),
            KeyCode::Char('0') => Some(Self::PanelZero),
            KeyCode::Char('1') => Some(Self::PanelOne),
            KeyCode::Char('2') => Some(Self::PanelTwo),
            KeyCode::Char('3') => Some(Self::PanelFour),
            KeyCode::Char('v' | 'V') => Some(Self::PanelVars),
            _ => None,
        }
    }
}

#[derive(Copy, Clone)]
enum VarsAction {
    Toggle,
    Copy,
    Delete,
}

impl VarsAction {
    const fn from_key(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Char(' ') => Some(Self::Toggle),
            KeyCode::Char('c' | 'C') => Some(Self::Copy),
            KeyCode::Char('d' | 'D') => Some(Self::Delete),
            _ => None,
        }
    }
}

fn handle_vars_action(app: &mut App, action: VarsAction) {
    match action {
        VarsAction::Toggle => {
            if let Some(var) = app.selected_managed_var() {
                let var = var.clone();
                app.toggle_managed_var_selection(&var);
            }
        }
        VarsAction::Copy => {
            let mut vars: Vec<String> = if app.managed_vars_selected.is_empty() {
                app.selected_managed_var().cloned().into_iter().collect()
            } else {
                app.managed_vars_selected.iter().cloned().collect()
            };

            if vars.is_empty() {
                app.command_log
                    .log_failure("Vars copy", "No vars selected".to_string());
                return;
            }

            vars.sort();
            let payload = vars.join(", ");

            match copy_to_clipboard(&payload) {
                Ok(()) => app.command_log.log_success("Vars copied", None),
                Err(err) => app.command_log.log_failure("Vars copy", err.to_string()),
            }
        }
        VarsAction::Delete => {
            let vars: Vec<String> = if app.managed_vars_selected.is_empty() {
                app.selected_managed_var().cloned().into_iter().collect()
            } else {
                app.managed_vars_selected.iter().cloned().collect()
            };

            if vars.is_empty() {
                app.command_log
                    .log_failure("Vars delete", "No vars selected".to_string());
                return;
            }

            let mut vars = vars;
            vars.sort();
            app.open_vars_delete_modal(vars);
        }
    }
}

fn copy_to_clipboard(value: &str) -> Result<()> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to launch pbcopy")?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(value.as_bytes())
            .context("Failed to write to pbcopy")?;
    }

    let status = child.wait().context("Failed to wait for pbcopy")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("pbcopy exited with status {status}")
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

#[allow(clippy::too_many_lines)]
fn handle_key_press(app: &mut App, key: KeyEvent) {
    if let Some(modal) = app.modal.clone() {
        match modal {
            crate::app::Modal::EnvVar { .. } => match key.code {
                KeyCode::Esc => app.close_modal(),
                KeyCode::Enter => {
                    let env_var_name = app.modal_env_var_name().unwrap_or("").to_string();
                    if env_var_name.is_empty() {
                        app.error_message =
                            Some("Environment variable name cannot be empty".to_string());
                        return;
                    }

                    let op_reference = match app.modal_field_reference() {
                        Some(reference) => reference.to_string(),
                        None => return,
                    };

                    let account_id = if let Some(account) = app.selected_account() {
                        account.account_uuid.clone()
                    } else {
                        app.error_message = Some("No account selected".to_string());
                        return;
                    };

                    match app.save_op_item_config(&env_var_name, &account_id, &op_reference) {
                        Ok(()) => {
                            app.command_log
                                .log_success(format!("Saved {env_var_name} to config"), None);
                            app.load_managed_vars();
                            if app.managed_vars_list_state.selected().is_none()
                                && !app.managed_vars.is_empty()
                            {
                                app.managed_vars_list_state.select(Some(0));
                            }
                            app.close_modal();
                        }
                        Err(e) => app.error_message = Some(e.to_string()),
                    }
                }
                KeyCode::Backspace => {
                    if let Some(env_var_name) = app.modal_env_var_name_mut() {
                        env_var_name.pop();
                        app.error_message = None;
                    }
                }
                KeyCode::Char(c) => {
                    if (c.is_ascii_alphanumeric() || c == '_')
                        && let Some(env_var_name) = app.modal_env_var_name_mut()
                    {
                        env_var_name.push(c.to_ascii_uppercase());
                        app.error_message = None;
                    }
                }
                _ => {}
            },
            crate::app::Modal::VarDeleteConfirm { .. } => match key.code {
                KeyCode::Esc | KeyCode::Char('n' | 'N') => app.close_modal(),
                KeyCode::Char('y' | 'Y') => {
                    if let Some(vars) = app.modal_vars_delete_targets() {
                        let vars = vars.to_vec();
                        match app.remove_managed_vars(&vars) {
                            Ok(()) => {
                                app.command_log.log_success("Vars removed", None);
                                app.close_modal();
                            }
                            Err(err) => app.error_message = Some(err.to_string()),
                        }
                    }
                }
                _ => {}
            },
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

    if app.focused_panel == FocusedPanel::VarsList
        && let Some(action) = VarsAction::from_key(key.code)
    {
        handle_vars_action(app, action);
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
                    if let Err(e) = app.set_default_account(&selected_account_id) {
                        app.command_log.log_failure(
                            "Failed to save default account configuration",
                            e.to_string(),
                        );
                    } else {
                        app.command_log
                            .log_success("Saved default account configuration", None);
                        AccountListNav.on_select(app);
                    }
                }
            }
            FocusedPanel::VaultList => {
                if let (Some(selected_account_id), Some(selected_vault_id)) = (
                    app.selected_account().map(|a| a.account_uuid.clone()),
                    app.vault_list_state
                        .selected()
                        .and_then(|idx| app.vaults.get(idx))
                        .map(|v| v.id.clone()),
                ) {
                    if let Err(e) = app.set_default_vault(&selected_account_id, &selected_vault_id)
                    {
                        app.command_log.log_failure(
                            "Failed to save default vault configuration",
                            e.to_string(),
                        );
                    } else {
                        app.command_log
                            .log_success("Saved default vault configuration", None);
                        VaultListNav.on_select(app);
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
            NavAction::PanelVars => {
                app.focused_panel = FocusedPanel::VarsList;
                if app.managed_vars_list_state.selected().is_none() && !app.managed_vars.is_empty()
                {
                    app.managed_vars_list_state.select(Some(0));
                }
            }
            nav_action => {
                let nav: &dyn ListNav = match app.focused_panel {
                    FocusedPanel::AccountList => &AccountListNav,
                    FocusedPanel::VaultList => &VaultListNav,
                    FocusedPanel::VaultItemList => &VaultItemListNav,
                    FocusedPanel::VaultItemDetail => &VaultItemDetailNav,
                    FocusedPanel::VarsList => &VarsListNav,
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
        state.select(Some(new_idx));
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

    fn on_select(&self, app: &mut App) {
        let idx = self.list_state(app).selected();
        self.set_selected_idx(app, idx);

        app.clear_search();
        app.vault_items.clear();
        app.filtered_item_indices.clear();
        app.selected_item_details = None;

        if let Err(e) = app.load_vaults() {
            app.error_message = Some(e.to_string());
        }

        if let Some(vault_idx) = app
            .selected_account()
            .map(|a| a.account_uuid.clone())
            .and_then(|account_id| {
                app.config
                    .as_ref()
                    .and_then(|c| c.default_vault_per_account.get(&account_id))
            })
            .and_then(|vault_id| app.vaults.iter().position(|v| &v.id == vault_id))
        {
            app.selected_vault_idx = Some(vault_idx);
            app.vault_list_state.select(Some(vault_idx));

            if let Err(e) = app.load_vault_items() {
                app.error_message = Some(e.to_string());
            }
        }

        app.focused_panel = FocusedPanel::VaultList;
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
        app.selected_item_details.as_ref().map_or(0, |d| {
            d.fields.iter().filter(|f| f.label != "notesPlain").count()
        })
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

struct VarsListNav;

impl ListNav for VarsListNav {
    fn len(&self, app: &App) -> usize {
        app.managed_vars.len()
    }

    fn list_state<'a>(&self, app: &'a mut App) -> &'a mut ListState {
        &mut app.managed_vars_list_state
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.managed_vars_list_state.select(idx);
    }

    fn on_select(&self, _app: &mut App) {
        // No-op: cursor position is enough for vars actions.
    }
}
