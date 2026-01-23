mod app;
mod command_log;
mod event;
mod ui;

use anyhow::Result;
use ratatui::DefaultTerminal;

use app::App;

fn run_app(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();

    app.load_config(None)?;
    app.load_vaults()?;
    app.load_accounts()?;

    if !app.accounts.is_empty() {
        app.selected_account_idx = Some(0);
    }

    if let Some(config) = &app.config {
        if let Some(default_vault_id) = &config.default_vault_id
            && app.selected_vault_idx.is_none()
        {
            app.selected_vault_idx = Some(0);
        }
    }

    if app.selected_account_idx.is_some() && app.selected_vault_idx.is_some() {
        app.load_vault_items()?;
    }

    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &mut app))?;
        event::handle_events(&mut app)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    ratatui::run(|terminal| run_app(terminal))?;
    Ok(())
}
