use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use confy;
use log::{debug, info};
use std::path::Path;

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
    handle_config_action_with_path(action, None)
}

fn handle_config_action_with_path(action: ConfigAction, config_path: Option<&Path>) -> Result<()> {
    debug!("Handling config action: {:?}", action);

    match action {
        ConfigAction::Get { key } => {
            info!("Getting config key: {}", key);

            let config: OpLoadConfig = if let Some(path) = config_path {
                confy::load_path(path).context("Failed to load configuration")?
            } else {
                confy::load("op_loader", None).context("Failed to load configuration")?
            };
            debug!("Config loaded successfully");

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
            info!("Getting config path");

            let config_path = confy::get_configuration_file_path("op_loader", None)
                .context("Failed to get config path")?
                .display()
                .to_string();

            debug!("Config path resolved to: {}", config_path);
            println!("{}", config_path);
            Ok(())
        }
    }
}

pub fn handle_env_injection() -> Result<()> {
    use std::process::Command;

    info!("Loading environment variable mappings");

    let config: OpLoadConfig =
        confy::load("op_loader", None).context("Failed to load configuration")?;
    debug!("Config loaded successfully");

    if config.inject_vars.is_empty() {
        info!("No environment variables configured");
        eprintln!("No environment variables configured. Use the TUI to add mappings.");
        return Ok(());
    }

    info!("Processing {} env var mappings", config.inject_vars.len());

    for (env_var_name, op_reference) in &config.inject_vars {
        debug!("Reading {} from {}", env_var_name, op_reference);

        let output = Command::new("op")
            .args(["read", op_reference])
            .output()
            .with_context(|| format!("Failed to run `op read {}`", op_reference))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("Failed to read {}: {}", op_reference, stderr.trim());
            eprintln!(
                "# Warning: Failed to read {} for {}: {}",
                op_reference, env_var_name, stderr
            );
            continue;
        }

        let value = String::from_utf8_lossy(&output.stdout);
        let value = value.trim();
        debug!("Successfully read value for {}", env_var_name);

        let escaped_value = value.replace("'", "'\"'\"'");
        println!("export {}='{}'", env_var_name, escaped_value);
    }

    info!("Finished processing env var mappings");
    Ok(())
}
