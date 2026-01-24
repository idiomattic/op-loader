use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use confy;

use crate::app::{App, OpLoadConfig};

#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Subcommand)]
enum Command {
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    Get {
        #[arg(short, long)]
        key: String,
    },
    Path,
}

fn handle_config_action(app: &mut App, action: ConfigAction) -> Result<()> {
    handle_config_action_with_config(app, action, None)
}

pub fn handle_config_action_with_config(
    app: &mut App,
    action: ConfigAction,
    config: Option<OpLoadConfig>,
) -> Result<()> {
    match action {
        ConfigAction::Get { key } => {
            if let Some(config) = config {
                match key.as_str() {
                    "default_account_id" => match &config.default_account_id {
                        Some(preferred_account) => println!("{}", preferred_account),
                        None => println!("(not set)"),
                    },
                    "default_vault_id" => match &config.default_vault_id {
                        Some(preferred_vault) => println!("{}", preferred_vault),
                        None => println!("(not set)"),
                    },
                    _ => anyhow::bail!("Unknown config key: '{}'.", key),
                }
                Ok(())
            } else {
                anyhow::bail!("Failed to load configuration")
            }
        }
        ConfigAction::Path => {
            let config_path = confy::get_configuration_file_path("op_loader", None)
                .context("Failed to get config path")?
                .display()
                .to_string();

            println!("{}", config_path);
            Ok(())
        }
    }
}
