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

    if let Some(idx) = app
        .config
        .as_ref()
        .and_then(|c| c.default_vault_id.as_ref())
        .and_then(|vault_id| app.vaults.iter().position(|v| &v.id == vault_id))
    {
        app.selected_vault_idx = Some(idx);
        app.vault_list_state.select(Some(idx));
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
