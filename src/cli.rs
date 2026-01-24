use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use confy;

use crate::app::OpLoadConfig;

#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Subcommand)]
pub enum Command {
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    Get {
        #[arg(short, long)]
        key: String,
    },
    Path,
}

pub fn handle_config_action(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Get { key } => {
            let config: OpLoadConfig =
                confy::load("op_loader", None).context("Failed to load configuration")?;

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
