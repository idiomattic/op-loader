use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::app::{InjectVarConfig, OpLoadConfig, TemplatedFile};
use crate::cache::{
    CacheKind, CacheRemoval, cache_dir, cache_file_for_account, ensure_cache_dir,
    remove_cache_for_account,
};

#[derive(Debug, Default, Serialize, Deserialize)]
struct LegacyOpLoadConfig {
    #[serde(default)]
    inject_vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    default_account_id: Option<String>,
    #[serde(default)]
    default_vault_per_account: std::collections::HashMap<String, String>,
    #[serde(default)]
    templated_files: std::collections::HashMap<String, TemplatedFile>,
}

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
    Env {
        /// Cache op inject output per account for this duration (e.g. 30s, 10m, 1h, 2d)
        #[arg(long, value_name = "DURATION")]
        cache_ttl: Option<String>,
    },
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    Template {
        #[command(subcommand)]
        action: TemplateAction,
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

#[derive(Subcommand, Debug)]
pub enum TemplateAction {
    /// Add a file to be managed as a template
    Add {
        /// Path to the file to manage (e.g., ~/.npmrc)
        path: String,
    },
    /// List all managed template files
    List,
    /// Stop managing a file as a template
    Remove {
        /// Path to the managed file
        path: String,
    },
    /// Render all templates (substituting variables)
    Render,
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Clear cached op inject output
    Clear {
        /// Clear cached output for a specific account ID
        #[arg(long)]
        account: Option<String>,
    },
}

pub fn handle_config_action(action: ConfigAction) -> Result<()> {
    handle_config_action_with_path(action, None)
}

fn handle_config_action_with_path(action: ConfigAction, config_path: Option<&Path>) -> Result<()> {
    debug!("Handling config action: {action:?}");

    match action {
        ConfigAction::Get { key } => {
            info!("Getting config key: {key}");

            let config: OpLoadConfig = if let Some(path) = config_path {
                confy::load_path(path).context("Failed to load configuration")?
            } else {
                confy::load("op_loader", None).context("Failed to load configuration")?
            };
            debug!("Config loaded successfully");

            match key.as_str() {
                "default_account_id" => match &config.default_account_id {
                    Some(preferred_account) => println!("{preferred_account}"),
                    None => println!("(not set)"),
                },
                _ => anyhow::bail!("Unknown config key: '{key}'."),
            }
            Ok(())
        }
        ConfigAction::Path => {
            info!("Getting config path");

            if let Some(path) = config_path {
                debug!("Config path (provided): {}", path.display());
                println!("{}", path.display());
            } else {
                let resolved_path = confy::get_configuration_file_path("op_loader", None)
                    .context("Failed to get config path")?
                    .display()
                    .to_string();

                debug!("Config path resolved to: {resolved_path}");
                println!("{resolved_path}");
            }
            Ok(())
        }
    }
}

pub fn handle_env_injection(cache_ttl: Option<&str>) -> Result<()> {
    info!("Loading environment variable mappings");

    let mut config: OpLoadConfig =
        confy::load("op_loader", None).context("Failed to load configuration")?;
    debug!("Config loaded successfully");

    if config.inject_vars.is_empty() {
        let legacy: LegacyOpLoadConfig =
            confy::load("op_loader", None).context("Failed to load configuration")?;

        if legacy.inject_vars.is_empty() {
            info!("No environment variables configured");
            eprintln!("No environment variables configured. Use the TUI to add mappings.");
            return Ok(());
        }

        eprintln!(
            "Warning: Legacy inject_vars format detected. Please re-add your environment variable mappings in the TUI."
        );
        config.inject_vars.clear();
        confy::store("op_loader", None, &config).context("Failed to save configuration")?;
    }

    if config.inject_vars.is_empty() {
        return Ok(());
    }

    info!("Processing {} env var mappings", config.inject_vars.len());

    let vars_by_account = group_vars_by_account(&config.inject_vars);

    let mut combined_output = String::new();

    let cache_ttl = cache_ttl.map(parse_duration).transpose()?.unwrap_or(None);

    for (account_id, vars) in vars_by_account {
        let mut input = String::new();
        for (env_var_name, var_config) in vars {
            use std::fmt::Write;
            writeln!(input, "export {env_var_name}='{}'", var_config.op_reference)
                .with_context(|| "Failed to write env export line")?;
        }

        if let Some(ttl) = cache_ttl {
            match read_cached_output(account_id, CacheKind::EnvInject, ttl) {
                Ok(CacheReadOutcome::Hit(cached)) => {
                    info!("Cache hit for account {account_id}");
                    combined_output.push_str(&cached);
                    continue;
                }
                Ok(CacheReadOutcome::Expired) => {
                    info!("Cache expired for account {account_id}");
                }
                Ok(CacheReadOutcome::Miss) => {
                    info!("Cache miss for account {account_id}");
                }
                Err(err) => {
                    eprintln!("# Warning: Failed to read cache for account {account_id}: {err}");
                }
            }
        }

        match run_op_inject(account_id, &input) {
            Ok(output) => {
                combined_output.push_str(&output);

                if cache_ttl.is_some()
                    && let Err(err) = write_cached_output(account_id, CacheKind::EnvInject, &output)
                {
                    eprintln!("# Warning: Failed to write cache for account {account_id}: {err}");
                }
            }
            Err(err) => {
                eprintln!("# Warning: Failed to inject secrets for account {account_id}: {err}");
            }
        }
    }

    print!("{combined_output}");

    info!("Finished processing env var mappings");

    if !config.templated_files.is_empty() {
        info!("Rendering {} template files", config.templated_files.len());
        render_templates(&config, cache_ttl)?;
    }

    Ok(())
}

fn run_op_inject(account_id: &str, input: &str) -> Result<String> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("op")
        .args(["inject", "--account", account_id])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to run `op inject --account {account_id}`"))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(input.as_bytes())
            .with_context(|| "Failed to write to op inject stdin")?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| "Failed to read op inject output")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("op inject failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_duration(input: &str) -> Result<Option<Duration>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed.len() < 2 {
        anyhow::bail!("Invalid duration '{input}'. Use a number followed by s, m, h, or d.");
    }

    let (value, unit) = trimmed.split_at(trimmed.len().saturating_sub(1));
    let amount: u64 = value
        .parse()
        .with_context(|| format!("Invalid duration value: {input}"))?;

    let seconds = match unit {
        "s" => amount,
        "m" => amount.saturating_mul(60),
        "h" => amount.saturating_mul(60 * 60),
        "d" => amount.saturating_mul(60 * 60 * 24),
        _ => anyhow::bail!("Invalid duration unit in '{input}'. Use s, m, h, or d."),
    };

    Ok(Some(Duration::from_secs(seconds)))
}

enum CacheReadOutcome {
    Hit(String),
    Miss,
    Expired,
}

fn read_cached_output(
    account_id: &str,
    kind: CacheKind,
    ttl: Duration,
) -> Result<CacheReadOutcome> {
    let path = cache_file_for_account(account_id, kind)?;
    let metadata = match std::fs::metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CacheReadOutcome::Miss);
        }
        Err(err) => {
            return Err(err)
                .with_context(|| format!("Failed to read cache metadata: {}", path.display()));
        }
    };

    let modified = metadata
        .modified()
        .with_context(|| format!("Failed to read cache mtime: {}", path.display()))?;

    let age = modified
        .elapsed()
        .unwrap_or_else(|_| Duration::from_secs(0));
    if age > ttl {
        return Ok(CacheReadOutcome::Expired);
    }

    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read cache file: {}", path.display()))?;
    Ok(CacheReadOutcome::Hit(contents))
}

fn write_cached_output(account_id: &str, kind: CacheKind, output: &str) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    ensure_cache_dir()?;
    let path = cache_file_for_account(account_id, kind)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .with_context(|| format!("Failed to open cache file for writing: {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .with_context(|| format!("Failed to set cache file permissions: {}", path.display()))?;
    }

    file.write_all(output.as_bytes())
        .with_context(|| format!("Failed to write cache file: {}", path.display()))?;
    Ok(())
}

fn get_templates_dir() -> Result<PathBuf> {
    let config_path = confy::get_configuration_file_path("op_loader", None)
        .context("Failed to get config path")?;
    let config_dir = config_path
        .parent()
        .context("Config path has no parent directory")?;
    Ok(config_dir.join("templates"))
}

fn expand_path(path: &str) -> Result<PathBuf> {
    let expanded = if let Some(suffix) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        PathBuf::from(home).join(suffix)
    } else {
        PathBuf::from(path)
    };

    if expanded.exists() {
        expanded
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", expanded.display()))
    } else {
        Ok(expanded)
    }
}

fn path_to_template_name(path: &Path) -> String {
    let filename = path.file_name().map_or_else(
        || "template".to_string(),
        |s| s.to_string_lossy().to_string(),
    );
    format!("{filename}.tmpl")
}

pub fn handle_template_action(action: TemplateAction) -> Result<()> {
    debug!("Handling template action: {action:?}");

    match action {
        TemplateAction::Add { path } => template_add(&path),
        TemplateAction::List => template_list(),
        TemplateAction::Remove { path } => template_remove(&path),
        TemplateAction::Render => {
            let config: OpLoadConfig =
                confy::load("op_loader", None).context("Failed to load configuration")?;
            render_templates(&config, None)
        }
    }
}

pub fn handle_cache_action(action: CacheAction) -> Result<()> {
    debug!("Handling cache action: {action:?}");

    match action {
        CacheAction::Clear { account } => match account {
            Some(account_id) => match remove_cache_for_account(&account_id) {
                Ok(CacheRemoval::Removed) => {
                    println!("Cleared cache for account {account_id}");
                }
                Ok(CacheRemoval::NotFound) => {
                    println!("No cache found for account {account_id}");
                }
                Err(err) => {
                    eprintln!("Warning: Failed to clear cache for account {account_id}: {err}");
                }
            },
            None => clear_all_caches()?,
        },
    }

    Ok(())
}

fn clear_all_caches() -> Result<()> {
    let dir = cache_dir()?;
    if !dir.exists() {
        println!("No cache directory found.");
        return Ok(());
    }

    let mut removed = 0usize;
    let mut failed = 0usize;
    let mut saw_file = false;
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("Failed to read cache directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(err) => {
                failed += 1;
                eprintln!("Warning: Failed to remove {}: {err}", path.display());
            }
        }
        saw_file = true;
    }

    if !saw_file {
        println!("No cache files found.");
        return Ok(());
    }

    println!(
        "Cleared {removed} cache file(s).{suffix}",
        suffix = if failed > 0 { " (some failures)" } else { "" }
    );
    Ok(())
}

fn template_add(path: &str) -> Result<()> {
    info!("Adding template for: {path}");

    let target_path = expand_path(path)?;
    let target_key = target_path.to_string_lossy().to_string();

    if !target_path.exists() {
        anyhow::bail!("File does not exist: {}", target_path.display());
    }

    let mut config: OpLoadConfig =
        confy::load("op_loader", None).context("Failed to load configuration")?;

    if config.templated_files.contains_key(&target_key) {
        anyhow::bail!(
            "File is already managed as a template: {}",
            target_path.display()
        );
    }

    let templates_dir = get_templates_dir()?;
    std::fs::create_dir_all(&templates_dir).with_context(|| {
        format!(
            "Failed to create templates directory: {}",
            templates_dir.display()
        )
    })?;

    let template_name = path_to_template_name(&target_path);
    let template_path = templates_dir.join(&template_name);

    let original_content =
        std::fs::read_to_string(&target_path).context("Failed to read source file")?;

    let var_names: Vec<String> = config
        .inject_vars
        .keys()
        .map(|k| format!("{{{{{k}}}}}"))
        .collect();

    let vars_comment = if var_names.is_empty() {
        "# op-loader: No variables configured yet. Use the TUI to add variables.\n".to_string()
    } else {
        format!(
            "# op-loader: Available variables: {}\n",
            var_names.join(", ")
        )
    };

    let template_content = format!("{vars_comment}{original_content}");
    std::fs::write(&template_path, &template_content)
        .with_context(|| format!("Failed to write template to {}", template_path.display()))?;

    config
        .templated_files
        .insert(target_key, TemplatedFile { template_name });
    confy::store("op_loader", None, &config).context("Failed to save configuration")?;

    println!("Added template for: {}", target_path.display());
    println!("Template stored at: {}", template_path.display());
    println!("\nAdd {{VAR_NAME}} placeholders to the template file.");
    println!("Use `op-loader template list` to see configured variables.");

    Ok(())
}

fn template_list() -> Result<()> {
    info!("Listing templates");

    let config: OpLoadConfig =
        confy::load("op_loader", None).context("Failed to load configuration")?;

    if config.templated_files.is_empty() {
        println!("No template files configured.");
        println!("\nAdd a template with: op-loader template add <path>");
        return Ok(());
    }

    let templates_dir = get_templates_dir()?;

    println!("Managed template files:\n");
    for (target_path, template_config) in &config.templated_files {
        let template_path = templates_dir.join(&template_config.template_name);
        let status = if template_path.exists() {
            "✓"
        } else {
            "✗ (missing)"
        };
        println!("  {status} {target_path}");
        println!("    └─ {}", template_path.display());
    }

    Ok(())
}

fn template_remove(path: &str) -> Result<()> {
    info!("Removing template for: {path}");

    let target_path = expand_path(path)?;
    let target_key = target_path.to_string_lossy().to_string();

    let mut config: OpLoadConfig =
        confy::load("op_loader", None).context("Failed to load configuration")?;

    let template_config = config
        .templated_files
        .remove(&target_key)
        .with_context(|| {
            format!(
                "File is not managed as a template: {}",
                target_path.display()
            )
        })?;

    let templates_dir = get_templates_dir()?;
    let template_path = templates_dir.join(&template_config.template_name);

    if template_path.exists() {
        std::fs::remove_file(&template_path)
            .with_context(|| format!("Failed to delete template: {}", template_path.display()))?;
        println!("Removed template: {}", template_path.display());
    } else {
        println!(
            "Removed config for: {} (template file was already missing)",
            target_path.display()
        );
    }

    confy::store("op_loader", None, &config).context("Failed to save configuration")?;

    Ok(())
}

fn render_templates(config: &OpLoadConfig, cache_ttl: Option<Duration>) -> Result<()> {
    let templates_dir = get_templates_dir()?;

    let mut resolved_vars: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    let vars_by_account = group_vars_by_account(&config.inject_vars);

    for (account_id, vars) in vars_by_account {
        let mut input = String::new();
        for (var_name, var_config) in vars {
            use std::fmt::Write;
            writeln!(input, "{var_name}: {}", var_config.op_reference)
                .with_context(|| "Failed to write template inject input")?;
        }

        let rendered = match load_template_output(account_id, &input, cache_ttl) {
            Ok(output) => output,
            Err(err) => {
                eprintln!("# Warning: Failed to inject secrets for account {account_id}: {err}");
                continue;
            }
        };

        for line in rendered.lines() {
            if let Some((var_name, value)) = line.split_once(": ") {
                resolved_vars.insert(var_name.to_string(), value.to_string());
            }
        }
    }

    for (target_path, template_config) in &config.templated_files {
        let template_path = templates_dir.join(&template_config.template_name);

        if !template_path.exists() {
            eprintln!(
                "# Warning: Template file not found for {}: {}",
                target_path,
                template_path.display()
            );
            continue;
        }

        debug!(
            "Rendering template: {} -> {}",
            template_path.display(),
            target_path
        );

        let template_content =
            std::fs::read_to_string(&template_path).context("Failed to read template file")?;

        let mut rendered: String = template_content
            .lines()
            .filter(|line| !line.starts_with("# op-loader:"))
            .collect::<Vec<_>>()
            .join("\n");

        if template_content.ends_with('\n') && !rendered.ends_with('\n') {
            rendered.push('\n');
        }

        for (var_name, value) in &resolved_vars {
            let placeholder = format!("{{{{{var_name}}}}}");
            rendered = rendered.replace(&placeholder, value);
        }

        let target = PathBuf::from(target_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(&target, &rendered)
            .with_context(|| format!("Failed to write to {target_path}"))?;

        info!("Rendered template: {target_path}");
    }

    Ok(())
}

fn fetch_template_output(
    account_id: &str,
    input: &str,
    cache_ttl: Option<Duration>,
) -> Result<String> {
    use std::process::Command;

    let output = Command::new("op")
        .args(["inject", "--account", account_id])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to run `op inject --account {account_id}`"))
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(input.as_bytes())
                    .with_context(|| "Failed to write to op inject stdin")?;
            }
            child
                .wait_with_output()
                .with_context(|| "Failed to read op inject output")
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("# Warning: Failed to inject secrets for account {account_id}: {stderr}");
        anyhow::bail!("op inject failed: {stderr}");
    }

    let rendered = String::from_utf8_lossy(&output.stdout).to_string();
    if cache_ttl.is_some()
        && let Err(err) = write_cached_output(account_id, CacheKind::TemplateRender, &rendered)
    {
        eprintln!("# Warning: Failed to write template cache for account {account_id}: {err}");
    }

    Ok(rendered)
}

fn load_template_output(
    account_id: &str,
    input: &str,
    cache_ttl: Option<Duration>,
) -> Result<String> {
    cache_ttl.map_or_else(
        || fetch_template_output(account_id, input, None),
        |ttl| match read_cached_output(account_id, CacheKind::TemplateRender, ttl) {
            Ok(CacheReadOutcome::Hit(cached)) => {
                info!("Template cache hit for account {account_id}");
                Ok(cached)
            }
            Ok(CacheReadOutcome::Expired) => {
                info!("Template cache expired for account {account_id}");
                fetch_template_output(account_id, input, Some(ttl))
            }
            Ok(CacheReadOutcome::Miss) => {
                info!("Template cache miss for account {account_id}");
                fetch_template_output(account_id, input, Some(ttl))
            }
            Err(err) => {
                eprintln!(
                    "# Warning: Failed to read template cache for account {account_id}: {err}"
                );
                fetch_template_output(account_id, input, Some(ttl))
            }
        },
    )
}

fn group_vars_by_account<'a>(
    inject_vars: &'a std::collections::HashMap<String, InjectVarConfig>,
) -> std::collections::BTreeMap<&'a str, Vec<(&'a str, &'a InjectVarConfig)>> {
    let mut vars_by_account: std::collections::BTreeMap<
        &'a str,
        Vec<(&'a str, &'a InjectVarConfig)>,
    > = std::collections::BTreeMap::new();

    for (var_name, var_config) in inject_vars {
        vars_by_account
            .entry(var_config.account_id.as_str())
            .or_default()
            .push((var_name.as_str(), var_config));
    }

    vars_by_account
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::cache::cache_path_for_account;
    use assert_fs::TempDir;
    use filetime::FileTime;

    fn write_cached_output_at(
        cache_root: &std::path::Path,
        account_id: &str,
        kind: CacheKind,
        output: &str,
    ) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        std::fs::create_dir_all(cache_root).with_context(|| {
            format!("Failed to create cache directory: {}", cache_root.display())
        })?;
        let path = cache_path_for_account(cache_root, account_id, kind);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .with_context(|| {
                format!("Failed to open cache file for writing: {}", path.display())
            })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata()?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms).with_context(|| {
                format!("Failed to set cache file permissions: {}", path.display())
            })?;
        }

        file.write_all(output.as_bytes())
            .with_context(|| format!("Failed to write cache file: {}", path.display()))?;
        Ok(())
    }

    fn read_cached_output_at(
        cache_root: &std::path::Path,
        account_id: &str,
        kind: CacheKind,
        ttl: Duration,
    ) -> Result<CacheReadOutcome> {
        let path = cache_path_for_account(cache_root, account_id, kind);
        let metadata = match std::fs::metadata(&path) {
            Ok(meta) => meta,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(CacheReadOutcome::Miss);
            }
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("Failed to read cache metadata: {}", path.display()));
            }
        };

        let modified = metadata
            .modified()
            .with_context(|| format!("Failed to read cache mtime: {}", path.display()))?;

        let age = modified
            .elapsed()
            .unwrap_or_else(|_| Duration::from_secs(0));
        if age > ttl {
            return Ok(CacheReadOutcome::Expired);
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read cache file: {}", path.display()))?;
        Ok(CacheReadOutcome::Hit(contents))
    }

    fn clear_all_caches_at(cache_root: &std::path::Path) -> Result<()> {
        if !cache_root.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(cache_root)
            .with_context(|| format!("Failed to read cache directory: {}", cache_root.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove cache file: {}", path.display()))?;
            }
        }
        Ok(())
    }

    #[test]
    fn cache_write_and_read_hit() {
        let temp_dir = TempDir::new().unwrap();
        let cache_root = temp_dir.path().join("op_loader");

        let output = "export FOO='bar'\n";
        write_cached_output_at(&cache_root, "account-1", CacheKind::EnvInject, output).unwrap();
        let result = read_cached_output_at(
            &cache_root,
            "account-1",
            CacheKind::EnvInject,
            Duration::from_secs(60),
        )
        .unwrap();

        match result {
            CacheReadOutcome::Hit(contents) => assert_eq!(contents, output),
            _ => panic!("Expected cache hit"),
        }
    }

    #[test]
    fn cache_read_expired_returns_expired() {
        let temp_dir = TempDir::new().unwrap();
        let cache_root = temp_dir.path().join("op_loader");

        write_cached_output_at(
            &cache_root,
            "account-2",
            CacheKind::EnvInject,
            "export TOKEN='old'\n",
        )
        .unwrap();
        let cache_path = cache_path_for_account(&cache_root, "account-2", CacheKind::EnvInject);
        let past = std::time::SystemTime::now() - Duration::from_secs(120);
        filetime::set_file_mtime(&cache_path, FileTime::from_system_time(past)).unwrap();

        let result = read_cached_output_at(
            &cache_root,
            "account-2",
            CacheKind::EnvInject,
            Duration::from_secs(60),
        )
        .unwrap();

        assert!(matches!(result, CacheReadOutcome::Expired));
    }

    #[test]
    fn cache_read_missing_returns_miss() {
        let temp_dir = TempDir::new().unwrap();
        let cache_root = temp_dir.path().join("op_loader");

        let result = read_cached_output_at(
            &cache_root,
            "missing-account",
            CacheKind::EnvInject,
            Duration::from_secs(60),
        )
        .unwrap();

        assert!(matches!(result, CacheReadOutcome::Miss));
    }

    #[test]
    fn cache_clear_removes_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let cache_root = temp_dir.path().join("op_loader");

        write_cached_output_at(
            &cache_root,
            "account-a",
            CacheKind::EnvInject,
            "export A=1\n",
        )
        .unwrap();
        std::fs::write(cache_root.join("extra-file.txt"), "extra").unwrap();
        std::fs::create_dir_all(cache_root.join("nested")).unwrap();

        clear_all_caches_at(&cache_root).unwrap();

        let remaining_files = std::fs::read_dir(cache_root)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_file())
            .count();
        assert_eq!(remaining_files, 0);
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use assert_fs::TempDir;

    #[test]
    fn config_get_default_account_id() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = OpLoadConfig {
            default_account_id: Some("test-account-123".to_string()),
            ..Default::default()
        };
        confy::store_path(&config_path, &config).unwrap();

        let result = handle_config_action_with_path(
            ConfigAction::Get {
                key: "default_account_id".to_string(),
            },
            Some(&config_path),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn config_get_unknown_key() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let result = handle_config_action_with_path(
            ConfigAction::Get {
                key: "nonexistent_key".to_string(),
            },
            Some(&config_path),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown config key")
        );
    }

    #[test]
    fn config_path_shows_custom_path() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let result = handle_config_action_with_path(ConfigAction::Path, Some(&config_path));

        assert!(result.is_ok());
    }

    #[test]
    fn config_get_when_file_does_not_exist_returns_not_set() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");

        let result = handle_config_action_with_path(
            ConfigAction::Get {
                key: "default_account_id".to_string(),
            },
            Some(&config_path),
        );

        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod template_tests {
    use super::*;

    mod path_to_template_name {
        use super::*;

        #[test]
        fn extracts_filename_from_path() {
            let path = Path::new("/Users/foo/.npmrc");
            let result = path_to_template_name(path);
            assert_eq!(result, ".npmrc.tmpl");
        }

        #[test]
        fn handles_simple_filename() {
            let path = Path::new("myfile.txt");
            let result = path_to_template_name(path);
            assert_eq!(result, "myfile.txt.tmpl");
        }

        #[test]
        fn handles_nested_path() {
            let path = Path::new("/home/user/.config/app/settings.json");
            let result = path_to_template_name(path);
            assert_eq!(result, "settings.json.tmpl");
        }
    }

    mod expand_path {
        use super::*;
        use std::env;

        #[test]
        fn expands_tilde_to_home() {
            let home = env::var("HOME").unwrap();
            let result = expand_path("~/.npmrc").unwrap();
            assert_eq!(result, PathBuf::from(format!("{}/.npmrc", home)));
        }

        #[test]
        fn preserves_absolute_path() {
            // For non-existent files, it returns the path as-is
            let result = expand_path("/some/absolute/path").unwrap();
            assert_eq!(result, PathBuf::from("/some/absolute/path"));
        }

        #[test]
        fn handles_relative_path() {
            let result = expand_path("relative/path").unwrap();
            assert_eq!(result, PathBuf::from("relative/path"));
        }
    }

    mod render_template_content {
        /// Helper to test template rendering logic without 1Password
        fn render_content(
            template: &str,
            vars: &std::collections::HashMap<String, String>,
        ) -> String {
            let mut rendered: String = template
                .lines()
                .filter(|line| !line.starts_with("# op-loader:"))
                .collect::<Vec<_>>()
                .join("\n");

            if template.ends_with('\n') && !rendered.ends_with('\n') {
                rendered.push('\n');
            }

            for (var_name, value) in vars {
                let placeholder = format!("{{{{{}}}}}", var_name);
                rendered = rendered.replace(&placeholder, value);
            }

            rendered
        }

        #[test]
        fn substitutes_single_variable() {
            let template = "token={{MY_TOKEN}}\n";
            let mut vars = std::collections::HashMap::new();
            vars.insert("MY_TOKEN".to_string(), "secret123".to_string());

            let result = render_content(template, &vars);
            assert_eq!(result, "token=secret123\n");
        }

        #[test]
        fn substitutes_multiple_variables() {
            let template = "user={{USER}}\npass={{PASS}}\n";
            let mut vars = std::collections::HashMap::new();
            vars.insert("USER".to_string(), "admin".to_string());
            vars.insert("PASS".to_string(), "secret".to_string());

            let result = render_content(template, &vars);
            assert_eq!(result, "user=admin\npass=secret\n");
        }

        #[test]
        fn strips_op_loader_comments() {
            let template = "# op-loader: Available variables: {{TOKEN}}\n# op-loader: This line too\ntoken={{TOKEN}}\n";
            let mut vars = std::collections::HashMap::new();
            vars.insert("TOKEN".to_string(), "abc".to_string());

            let result = render_content(template, &vars);
            assert_eq!(result, "token=abc\n");
        }

        #[test]
        fn preserves_other_comments() {
            let template = "# This is a regular comment\ntoken={{TOKEN}}\n";
            let mut vars = std::collections::HashMap::new();
            vars.insert("TOKEN".to_string(), "xyz".to_string());

            let result = render_content(template, &vars);
            assert_eq!(result, "# This is a regular comment\ntoken=xyz\n");
        }

        #[test]
        fn handles_same_var_multiple_times() {
            let template = "first={{VAR}} second={{VAR}}\n";
            let mut vars = std::collections::HashMap::new();
            vars.insert("VAR".to_string(), "value".to_string());

            let result = render_content(template, &vars);
            assert_eq!(result, "first=value second=value\n");
        }

        #[test]
        fn leaves_unmatched_placeholders() {
            let template = "token={{UNKNOWN}}\n";
            let vars = std::collections::HashMap::new();

            let result = render_content(template, &vars);
            assert_eq!(result, "token={{UNKNOWN}}\n");
        }

        #[test]
        fn preserves_trailing_newline() {
            let template = "content\n";
            let vars = std::collections::HashMap::new();

            let result = render_content(template, &vars);
            assert!(result.ends_with('\n'));
        }

        #[test]
        fn handles_empty_template() {
            let template = "";
            let vars = std::collections::HashMap::new();

            let result = render_content(template, &vars);
            assert_eq!(result, "");
        }
    }
}
