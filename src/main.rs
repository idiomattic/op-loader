mod app;
mod cli;
mod command_log;
mod event;
mod ui;

use anyhow::Result;
use clap::Parser;
use env_logger;
use ratatui::DefaultTerminal;

use app::App;
use cli::{Cli, Command};

fn run_app(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();

    app.load_config(None)?;
    app.load_vaults()?;
    app.load_accounts()?;

    if let Some(account_idx) = app
        .config
        .as_ref()
        .and_then(|c| c.default_account_id.as_ref())
        .and_then(|account_id| {
            app.accounts
                .iter()
                .position(|a| &a.account_uuid == account_id)
        })
    {
        app.selected_account_idx = Some(account_idx);
        app.account_list_state.select(Some(account_idx));
    } else {
        app.selected_account_idx = Some(0);
    }

    if let Some(vault_idx) = app
        .config
        .as_ref()
        .and_then(|c| c.default_vault_id.as_ref())
        .and_then(|vault_id| app.vaults.iter().position(|v| &v.id == vault_id))
    {
        app.selected_vault_idx = Some(vault_idx);
        app.vault_list_state.select(Some(vault_idx));
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
    let args = Cli::parse();

    env_logger::Builder::new()
        .filter_level(args.verbosity.into())
        .init();

    match args.command {
        Some(Command::Config { action }) => cli::handle_config_action(action)?,
        Some(Command::Env) => cli::handle_env_injection()?,
        None => ratatui::run(|terminal| run_app(terminal))?,
    };
    Ok(())
}
