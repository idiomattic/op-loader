use anyhow::Result;
use clap::{Parser, Subcommand};

use super::app::OpLoadConfig;

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
    Set {
        #[arg(short, long)]
        key: String,
    },
    Get {
        #[arg(short, long)]
        key: String,
    },
    Path,
}

fn handle_config_action(action: ConfigAction) -> Result<()> {
    handle_config_action_with_config(action, None)
}

pub fn handle_config_action_with_config(
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
        ConfigAction::Set { key } => {}
        ConfigAction::Path => {}
    }
}
