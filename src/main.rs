mod app;
mod event;
mod ui;

use ratatui::DefaultTerminal;
use std::io;

use app::App;

fn run_app(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new();

    if let Err(e) = app.load_vaults() {
        eprintln!("Failed to load vaults: {}", e);
        return Err(e);
    }

    if let Err(e) = app.load_accounts() {
        eprintln!("Failed to load accounts: {}", e);
        return Err(e);
    }

    if !app.accounts.is_empty() {
        app.selected_account_idx = Some(0);
    }

    if app.selected_account_idx.is_some() && app.selected_vault_idx.is_some() {
        if let Err(e) = app.load_vault_items() {
            eprintln!("Failed to load vault items: {}", e);
            return Err(e);
        }
    }

    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &mut app))?;
        event::handle_events(&mut app)?;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| run_app(terminal))
}
