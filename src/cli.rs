use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{debug, info};
use std::path::{Path, PathBuf};

use crate::app::{OpLoadConfig, TemplatedFile};

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
                _ => anyhow::bail!("Unknown config key: '{}'.", key),
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

                debug!("Config path resolved to: {}", resolved_path);
                println!("{}", resolved_path);
            }
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

    // Also render templates
    if !config.templated_files.is_empty() {
        info!("Rendering {} template files", config.templated_files.len());
        render_templates(&config)?;
    }

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
    let expanded = if path.starts_with("~/") {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        PathBuf::from(home).join(&path[2..])
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
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "template".to_string());
    format!("{}.tmpl", filename)
}

pub fn handle_template_action(action: TemplateAction) -> Result<()> {
    debug!("Handling template action: {:?}", action);

    match action {
        TemplateAction::Add { path } => template_add(&path),
        TemplateAction::List => template_list(),
        TemplateAction::Remove { path } => template_remove(&path),
        TemplateAction::Render => {
            let config: OpLoadConfig =
                confy::load("op_loader", None).context("Failed to load configuration")?;
            render_templates(&config)
        }
    }
}

fn template_add(path: &str) -> Result<()> {
    info!("Adding template for: {}", path);

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
        .map(|k| format!("{{{{{}}}}}", k))
        .collect();

    let vars_comment = if var_names.is_empty() {
        "# op-loader: No variables configured yet. Use the TUI to add variables.\n".to_string()
    } else {
        format!(
            "# op-loader: Available variables: {}\n",
            var_names.join(", ")
        )
    };

    let template_content = format!("{}{}", vars_comment, original_content);
    std::fs::write(&template_path, &template_content)
        .with_context(|| format!("Failed to write template to {}", template_path.display()))?;

    config.templated_files.insert(
        target_key,
        TemplatedFile {
            template_name: template_name.clone(),
        },
    );
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
        println!("  {} {}", status, target_path);
        println!("    └─ {}", template_path.display());
    }

    Ok(())
}

fn template_remove(path: &str) -> Result<()> {
    info!("Removing template for: {}", path);

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

fn render_templates(config: &OpLoadConfig) -> Result<()> {
    use std::process::Command;

    let templates_dir = get_templates_dir()?;

    let mut resolved_vars: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for (var_name, op_reference) in &config.inject_vars {
        debug!("Resolving {} from {}", var_name, op_reference);

        let output = Command::new("op")
            .args(["read", op_reference])
            .output()
            .with_context(|| format!("Failed to run `op read {}`", op_reference))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "# Warning: Failed to read {} for {}: {}",
                op_reference, var_name, stderr
            );
            continue;
        }

        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        resolved_vars.insert(var_name.clone(), value);
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
            let placeholder = format!("{{{{{}}}}}", var_name);
            rendered = rendered.replace(&placeholder, value);
        }

        let target = PathBuf::from(target_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        std::fs::write(&target, &rendered)
            .with_context(|| format!("Failed to write to {}", target_path))?;

        info!("Rendered template: {}", target_path);
    }

    Ok(())
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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown config key"));
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
