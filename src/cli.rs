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
    Env,
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

pub fn handle_env_injection() -> Result<()> {
    use std::process::Command;

    let config: OpLoadConfig =
        confy::load("op_loader", None).context("Failed to load configuration")?;

    if config.inject_vars.is_empty() {
        eprintln!("No environment variables configured. Use the TUI to add mappings.");
        return Ok(());
    }

    for (env_var_name, op_reference) in &config.inject_vars {
        let output = Command::new("op")
            .args(["read", op_reference])
            .output()
            .with_context(|| format!("Failed to run `op read {}`", op_reference))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "# Warning: Failed to read {} for {}: {}",
                op_reference, env_var_name, stderr
            );
            continue;
        }

        let value = String::from_utf8_lossy(&output.stdout);
        let value = value.trim();

        let escaped_value = value.replace("'", "'\"'\"'");
        println!("export {}='{}'", env_var_name, escaped_value);
    }

    Ok(())
}
