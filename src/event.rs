use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::io;

use crate::app::{App, FocusedPanel};

pub fn handle_events(app: &mut App) -> io::Result<()> {
    if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
            handle_key_press(app, key);
        }
    }
    Ok(())
}

fn handle_key_press(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
        }
        _ => match app.focused_panel {
            FocusedPanel::VaultList => handle_vault_list_input(app, key),
            FocusedPanel::AccountList => {}
        },
    }
}

fn handle_vault_list_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            if !app.vaults.is_empty() {
                let idx = app.vault_list_state.selected().unwrap_or(0);
                let new_idx = if idx == 0 {
                    app.vaults.len() - 1
                } else {
                    idx - 1
                };
                app.vault_list_state.select(Some(new_idx));
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            if !app.vaults.is_empty() {
                let idx = app.vault_list_state.selected().unwrap_or(0);
                let new_idx = if idx == app.vaults.len() - 1 {
                    0
                } else {
                    idx + 1
                };
                app.vault_list_state.select(Some(new_idx));
            }
        }
        KeyCode::Enter => {
            let idx = app.vault_list_state.selected().unwrap_or(0);
            app.selected_vault_idx = Some(idx);
        }
        _ => {}
    }
}
