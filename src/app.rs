use anyhow::{Context, Result, bail};
use confy;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Command};

use crate::command_log::CommandLog;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OpLoadConfig {
    inject_vars: HashMap<String, String>,
}

pub struct App {
    pub config: Option<OpLoadConfig>,

    pub should_quit: bool,
    pub focused_panel: FocusedPanel,
    pub error_message: Option<String>,
    pub command_log: CommandLog,

    pub accounts: Vec<Account>,
    pub account_list_state: ListState,
    pub selected_account_idx: Option<usize>,

    pub vaults: Vec<Vault>,
    pub vault_list_state: ListState,
    pub selected_vault_idx: Option<usize>,

    pub vault_items: Vec<VaultItem>,
    pub vault_item_list_state: ListState,
    pub selected_vault_item_idx: Option<usize>,
    pub selected_item_details: Option<VaultItemDetails>,

    pub item_detail_list_state: ListState,
    pub selected_field_idx: Option<usize>,

    pub search_query: String,
    pub search_active: bool,
    pub filtered_item_indices: Vec<usize>,

    pub modal_open: bool,
    pub modal_env_var_name: String,
    pub modal_field_reference: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let app = Self {
            config: None,

            should_quit: false,
            focused_panel: FocusedPanel::VaultList,
            error_message: None,
            command_log: CommandLog::default(),

            vaults: Vec::new(),
            vault_list_state: ListState::default(),
            selected_vault_idx: None,

            accounts: Vec::new(),
            account_list_state: ListState::default(),
            selected_account_idx: None,

            vault_items: Vec::new(),
            vault_item_list_state: ListState::default(),
            selected_vault_item_idx: None,
            selected_item_details: None,

            item_detail_list_state: ListState::default(),
            selected_field_idx: None,

            search_query: String::new(),
            search_active: false,
            filtered_item_indices: Vec::new(),

            modal_open: false,
            modal_env_var_name: String::new(),
            modal_field_reference: None,
        };

        app
    }

    pub fn load_config(&mut self, config_path: Option<&std::path::Path>) -> Result<()> {
        let config: OpLoadConfig = if let Some(path) = config_path {
            confy::load_path(path).context("Failed to load configuration")?
        } else {
            confy::load("op_loader", None).context("Failed to load configuration")?
        };

        self.config = Some(config);

        Ok(())
    }

    pub fn save_config(&mut self, var_name: &str, op_reference: &str) -> Result<()> {
        if let Some(config) = &mut self.config {
            config
                .inject_vars
                .insert(var_name.to_string(), op_reference.to_string());
            confy::store("op_loader", None, &*config).context("Failed to save configuration")?;
        } else {
            anyhow::bail!("Configuration can't be saved because it is not loaded");
        }

        Ok(())
    }

    fn run_op_command(&mut self, args: &[&str]) -> Result<Vec<u8>> {
        let cmd_str = format!("op {}", args.join(" "));

        let output = Command::new("op")
            .args(args)
            .output()
            .context("Failed to execute op command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            self.command_log.log_failure(&cmd_str, &stderr);
            bail!("`{}` failed: {}", cmd_str, stderr);
        }

        Ok(output.stdout)
    }

    pub fn load_vaults(&mut self) -> Result<()> {
        let stdout = self.run_op_command(&["vault", "list", "--format", "json"])?;

        let vaults: Vec<Vault> =
            serde_json::from_slice(&stdout).context("Failed to parse vault list JSON")?;

        self.command_log
            .log_success("op vault list", Some(vaults.len()));

        self.vaults = vaults;

        if !self.vaults.is_empty() {
            self.vault_list_state.select(Some(0));
        }

        Ok(())
    }

    pub fn selected_vault(&self) -> Option<&Vault> {
        self.selected_vault_idx.and_then(|idx| self.vaults.get(idx))
    }

    pub fn load_accounts(&mut self) -> Result<()> {
        let stdout = self.run_op_command(&["account", "list", "--format", "json"])?;

        let accounts: Vec<Account> =
            serde_json::from_slice(&stdout).context("Failed to parse account list JSON")?;

        self.command_log
            .log_success("op account list", Some(accounts.len()));

        self.accounts = accounts;

        if !self.accounts.is_empty() {
            self.account_list_state.select(Some(0));
        }

        Ok(())
    }

    pub fn load_vault_items(&mut self) -> Result<()> {
        if self.selected_account_idx.is_none() || self.selected_vault_idx.is_none() {
            bail!("Cannot list vault items when account/vault are not selected");
        }

        let selected_vault_name = &self.selected_vault().unwrap().name.clone();

        let stdout = self.run_op_command(&[
            "item",
            "list",
            "--vault",
            selected_vault_name,
            "--format",
            "json",
        ])?;

        let vault_items: Vec<VaultItem> =
            serde_json::from_slice(&stdout).context("Failed to parse vault items JSON")?;

        self.command_log.log_success(
            format!("op item list --vault {}", selected_vault_name),
            Some(vault_items.len()),
        );

        self.vault_items = vault_items;
        self.update_filtered_items();

        if !self.filtered_item_indices.is_empty() {
            self.vault_item_list_state.select(Some(0));
        }

        Ok(())
    }

    pub fn update_filtered_items(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_item_indices = (0..self.vault_items.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(usize, i64)> = self
                .vault_items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    matcher
                        .fuzzy_match(&item.title, &self.search_query)
                        .map(|score| (idx, score))
                })
                .collect();
            scored.sort_by(|a, b| b.1.cmp(&a.1)); // highest score first
            self.filtered_item_indices = scored.into_iter().map(|(idx, _)| idx).collect();
        }

        if !self.filtered_item_indices.is_empty() {
            self.vault_item_list_state.select(Some(0));
        } else {
            self.vault_item_list_state.select(None);
        }
        self.selected_vault_item_idx = None;
        self.selected_item_details = None;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_active = false;
        self.update_filtered_items();
    }

    pub fn load_item_details(&mut self, item_id: &str) -> Result<()> {
        let vault_name = self.selected_vault().unwrap().name.clone();

        let stdout = self.run_op_command(&[
            "item",
            "get",
            item_id,
            "--vault",
            &vault_name,
            "--format",
            "json",
        ])?;

        let details: VaultItemDetails =
            serde_json::from_slice(&stdout).context("Failed to parse item details JSON")?;

        self.command_log.log_success(
            format!("op item get {}", item_id),
            Some(details.fields.len()),
        );

        self.selected_item_details = Some(details);
        Ok(())
    }

    pub fn open_modal(&mut self, field_reference: String) {
        self.modal_open = true;
        self.modal_env_var_name.clear();
        self.modal_field_reference = Some(field_reference);
    }

    pub fn close_modal(&mut self) {
        self.modal_open = false;
        self.modal_env_var_name.clear();
        self.modal_field_reference = None;
        self.error_message = None;
    }

    pub fn modal_selected_field(&self) -> Option<&ItemField> {
        let details = self.selected_item_details.as_ref()?;
        let reference = self.modal_field_reference.as_ref()?;
        details.fields.iter().find(|f| &f.reference == reference)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vault {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub email: String,
    pub user_uuid: String,
    pub account_uuid: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemUrl {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub primary: bool,
    pub href: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultItem {
    pub id: String,
    pub title: String,
    pub category: String,
    #[serde(default)]
    pub additional_information: Option<String>,
    #[serde(default)]
    pub urls: Vec<ItemUrl>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultItemDetails {
    pub id: String,
    pub title: String,
    pub category: String,
    #[serde(default)]
    pub fields: Vec<ItemField>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemField {
    pub label: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub field_type: String,
    pub reference: String,
    #[serde(default)]
    pub section: Option<FieldSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldSection {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(PartialEq)]
pub enum FocusedPanel {
    AccountList,
    VaultList,
    VaultItemList,
    VaultItemDetail,
}
