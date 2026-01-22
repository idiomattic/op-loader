use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::ListState;
use std::io;

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

pub fn handle_events(app: &mut App) -> io::Result<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            handle_key_press(app, key);
        }
    }
    Ok(())
}

fn handle_key_press(app: &mut App, key: KeyEvent) {
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

    if key.code == KeyCode::Char('/') && app.focused_panel == FocusedPanel::VaultItemList {
        app.search_active = true;
        return;
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

    fn selected_idx(&self, app: &App) -> Option<usize>;
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

    fn selected_idx(&self, app: &App) -> Option<usize> {
        app.selected_account_idx
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

    fn selected_idx(&self, app: &App) -> Option<usize> {
        app.selected_vault_idx
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.selected_vault_idx = idx;
    }

    fn on_select(&self, app: &mut App) {
        let idx = self.list_state(app).selected();
        self.set_selected_idx(app, idx);

        // Clear search when selecting a new vault
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

    fn selected_idx(&self, app: &App) -> Option<usize> {
        app.selected_vault_item_idx
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.selected_vault_item_idx = idx;
    }

    fn on_select(&self, app: &mut App) {
        let list_idx = self.list_state(app).selected();
        self.set_selected_idx(app, list_idx);

        if let Some(list_idx) = list_idx {
            if let Some(&real_idx) = app.filtered_item_indices.get(list_idx) {
                if let Some(item) = app.vault_items.get(real_idx) {
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

    fn selected_idx(&self, app: &App) -> Option<usize> {
        app.selected_field_idx
    }

    fn set_selected_idx(&self, app: &mut App, idx: Option<usize>) {
        app.selected_field_idx = idx;
    }
}
