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
        },
    }
}

fn handle_vault_list_input(app: &mut App, key: KeyEvent) {}
