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
    if let Some(action) = NavAction::from_key(key.code) {
        match action {
            NavAction::Quit => app.should_quit = true,
            NavAction::PanelZero => app.focused_panel = FocusedPanel::AccountList,
            NavAction::PanelOne => app.focused_panel = FocusedPanel::VaultList,
            NavAction::PanelTwo => app.focused_panel = FocusedPanel::VaultItemList,
            nav_action => {
                let nav: &dyn ListNav = match app.focused_panel {
                    FocusedPanel::AccountList => &AccountListNav,
                    FocusedPanel::VaultList => &VaultListNav,
                    FocusedPanel::VaultItemList => &VaultItemListNav,
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

        if let Err(e) = app.load_vault_items() {
            app.error_message = Some(e.to_string());
        }
    }
}

struct VaultItemListNav;
impl ListNav for VaultItemListNav {
    fn len(&self, app: &App) -> usize {
        app.vault_items.len()
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
}
