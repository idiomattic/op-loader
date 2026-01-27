use anyhow::{bail, Context, Result};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Command};

use crate::command_log::CommandLog;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct OpLoadConfig {
    pub inject_vars: HashMap<String, String>,
    pub default_vault_id: Option<String>,
    pub default_account_id: Option<String>,
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
        Self {
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
        }
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

    pub fn save_op_item_config(&mut self, var_name: &str, op_reference: &str) -> Result<()> {
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

    pub fn set_default_vault(&mut self, vault_id: &str) -> Result<()> {
        if let Some(config) = &mut self.config {
            config.default_vault_id = Some(vault_id.to_string());
            confy::store("op_loader", None, &*config).context("Failed to save configuration")?;
        } else {
            anyhow::bail!("Configuration can't be saved because it is not loaded");
        }

        Ok(())
    }

    pub fn set_default_account(&mut self, account_id: &str) -> Result<()> {
        if let Some(config) = &mut self.config {
            config.default_account_id = Some(account_id.to_string());
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
        let account_uuid = self.selected_account().map(|a| a.account_uuid.clone());

        let stdout = if let Some(ref uuid) = account_uuid {
            self.run_op_command(&["vault", "list", "--account", uuid, "--format", "json"])?
        } else {
            self.run_op_command(&["vault", "list", "--format", "json"])?
        };

        let vaults: Vec<Vault> =
            serde_json::from_slice(&stdout).context("Failed to parse vault list JSON")?;

        self.command_log
            .log_success("op vault list", Some(vaults.len()));

        self.vaults = vaults;
        self.selected_vault_idx = None;

        if !self.vaults.is_empty() {
            self.vault_list_state.select(Some(0));
        } else {
            self.vault_list_state.select(None);
        }

        Ok(())
    }

    pub fn selected_vault(&self) -> Option<&Vault> {
        self.selected_vault_idx.and_then(|idx| self.vaults.get(idx))
    }

    pub fn selected_account(&self) -> Option<&Account> {
        self.selected_account_idx
            .and_then(|idx| self.accounts.get(idx))
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

        let account_id = self.selected_account().unwrap().account_uuid.clone();
        let vault_id = self.selected_vault().unwrap().id.clone();

        let stdout = self.run_op_command(&[
            "item",
            "list",
            "--account",
            &account_id,
            "--vault",
            &vault_id,
            "--format",
            "json",
        ])?;

        let vault_items: Vec<VaultItem> =
            serde_json::from_slice(&stdout).context("Failed to parse vault items JSON")?;

        self.command_log.log_success(
            format!("op item list --vault {}", vault_id),
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
        let account_id = self.selected_account().unwrap().account_uuid.clone();
        let vault_id = self.selected_vault().unwrap().id.clone();

        let stdout = self.run_op_command(&[
            "item",
            "get",
            item_id,
            "--account",
            &account_id,
            "--vault",
            &vault_id,
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
    #[allow(dead_code)]
    pub user_uuid: String,
    pub account_uuid: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub category: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub additional_information: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub urls: Vec<ItemUrl>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultItemDetails {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub section: Option<FieldSection>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vault_item(id: &str, title: &str) -> VaultItem {
        VaultItem {
            id: id.to_string(),
            title: title.to_string(),
            category: "LOGIN".to_string(),
            additional_information: None,
            urls: vec![],
        }
    }

    fn make_item_field(label: &str, reference: &str) -> ItemField {
        ItemField {
            label: label.to_string(),
            value: Some("secret-value".to_string()),
            field_type: "CONCEALED".to_string(),
            reference: reference.to_string(),
            section: None,
        }
    }

    mod update_filtered_items {
        use super::*;

        #[test]
        fn empty_query_returns_all_items() {
            let mut app = App::new();
            app.vault_items = vec![
                make_vault_item("1", "GitHub Token"),
                make_vault_item("2", "AWS Secret"),
                make_vault_item("3", "Database Password"),
            ];
            app.search_query = String::new();

            app.update_filtered_items();

            assert_eq!(app.filtered_item_indices, vec![0, 1, 2]);
        }

        #[test]
        fn filters_by_fuzzy_match() {
            let mut app = App::new();
            app.vault_items = vec![
                make_vault_item("1", "GitHub Token"),
                make_vault_item("2", "AWS Secret"),
                make_vault_item("3", "GitLab Token"),
            ];
            app.search_query = "git".to_string();

            app.update_filtered_items();

            assert_eq!(app.filtered_item_indices.len(), 2);
            assert!(app.filtered_item_indices.contains(&0)); // GitHub
            assert!(app.filtered_item_indices.contains(&2)); // GitLab
        }

        #[test]
        fn no_matches_returns_empty() {
            let mut app = App::new();
            app.vault_items = vec![
                make_vault_item("1", "GitHub Token"),
                make_vault_item("2", "AWS Secret"),
            ];
            app.search_query = "zzzzz".to_string();

            app.update_filtered_items();

            assert!(app.filtered_item_indices.is_empty());
            assert!(app.vault_item_list_state.selected().is_none());
        }

        #[test]
        fn selects_first_item_when_results_exist() {
            let mut app = App::new();
            app.vault_items = vec![
                make_vault_item("1", "GitHub Token"),
                make_vault_item("2", "AWS Secret"),
            ];
            app.search_query = String::new();

            app.update_filtered_items();

            assert_eq!(app.vault_item_list_state.selected(), Some(0));
        }

        #[test]
        fn clears_selected_item_details() {
            let mut app = App::new();
            app.vault_items = vec![make_vault_item("1", "GitHub Token")];
            app.selected_vault_item_idx = Some(0);
            app.selected_item_details = Some(VaultItemDetails {
                id: "1".to_string(),
                title: "GitHub Token".to_string(),
                category: "LOGIN".to_string(),
                fields: vec![],
            });

            app.update_filtered_items();

            assert!(app.selected_vault_item_idx.is_none());
            assert!(app.selected_item_details.is_none());
        }

        #[test]
        fn empty_vault_items_returns_empty() {
            let mut app = App::new();
            app.vault_items = vec![];
            app.search_query = "test".to_string();

            app.update_filtered_items();

            assert!(app.filtered_item_indices.is_empty());
        }
    }

    mod clear_search {
        use super::*;

        #[test]
        fn clears_query_and_deactivates() {
            let mut app = App::new();
            app.search_query = "some search".to_string();
            app.search_active = true;

            app.clear_search();

            assert!(app.search_query.is_empty());
            assert!(!app.search_active);
        }

        #[test]
        fn resets_filtered_items_to_all() {
            let mut app = App::new();
            app.vault_items = vec![
                make_vault_item("1", "GitHub Token"),
                make_vault_item("2", "AWS Secret"),
            ];
            app.search_query = "git".to_string();
            app.update_filtered_items();

            app.clear_search();

            assert_eq!(app.filtered_item_indices, vec![0, 1]);
        }
    }

    mod open_modal {
        use super::*;

        #[test]
        fn sets_modal_state() {
            let mut app = App::new();
            let reference = "op://vault/item/field".to_string();

            app.open_modal(reference.clone());

            assert!(app.modal_open);
            assert_eq!(app.modal_field_reference, Some(reference));
            assert!(app.modal_env_var_name.is_empty());
        }

        #[test]
        fn clears_previous_env_var_name() {
            let mut app = App::new();
            app.modal_env_var_name = "OLD_VAR".to_string();

            app.open_modal("op://vault/item/field".to_string());

            assert!(app.modal_env_var_name.is_empty());
        }
    }

    mod close_modal {
        use super::*;

        #[test]
        fn resets_all_modal_state() {
            let mut app = App::new();
            app.modal_open = true;
            app.modal_env_var_name = "MY_VAR".to_string();
            app.modal_field_reference = Some("op://vault/item/field".to_string());
            app.error_message = Some("some error".to_string());

            app.close_modal();

            assert!(!app.modal_open);
            assert!(app.modal_env_var_name.is_empty());
            assert!(app.modal_field_reference.is_none());
            assert!(app.error_message.is_none());
        }
    }

    mod modal_selected_field {
        use super::*;

        #[test]
        fn returns_matching_field() {
            let mut app = App::new();
            let reference = "op://vault/item/password".to_string();
            app.selected_item_details = Some(VaultItemDetails {
                id: "1".to_string(),
                title: "Test Item".to_string(),
                category: "LOGIN".to_string(),
                fields: vec![
                    make_item_field("username", "op://vault/item/username"),
                    make_item_field("password", "op://vault/item/password"),
                ],
            });
            app.modal_field_reference = Some(reference);

            let field = app.modal_selected_field();

            assert!(field.is_some());
            assert_eq!(field.unwrap().label, "password");
        }

        #[test]
        fn returns_none_when_no_details() {
            let mut app = App::new();
            app.selected_item_details = None;
            app.modal_field_reference = Some("op://vault/item/field".to_string());

            assert!(app.modal_selected_field().is_none());
        }

        #[test]
        fn returns_none_when_no_reference() {
            let mut app = App::new();
            app.selected_item_details = Some(VaultItemDetails {
                id: "1".to_string(),
                title: "Test Item".to_string(),
                category: "LOGIN".to_string(),
                fields: vec![make_item_field("password", "op://vault/item/password")],
            });
            app.modal_field_reference = None;

            assert!(app.modal_selected_field().is_none());
        }

        #[test]
        fn returns_none_when_reference_not_found() {
            let mut app = App::new();
            app.selected_item_details = Some(VaultItemDetails {
                id: "1".to_string(),
                title: "Test Item".to_string(),
                category: "LOGIN".to_string(),
                fields: vec![make_item_field("password", "op://vault/item/password")],
            });
            app.modal_field_reference = Some("op://vault/item/nonexistent".to_string());

            assert!(app.modal_selected_field().is_none());
        }
    }

    mod selected_vault {
        use super::*;

        #[test]
        fn returns_vault_at_index() {
            let mut app = App::new();
            app.vaults = vec![
                Vault {
                    id: "v1".to_string(),
                    name: "Personal".to_string(),
                },
                Vault {
                    id: "v2".to_string(),
                    name: "Work".to_string(),
                },
            ];
            app.selected_vault_idx = Some(1);

            let vault = app.selected_vault();

            assert!(vault.is_some());
            assert_eq!(vault.unwrap().name, "Work");
        }

        #[test]
        fn returns_none_when_no_selection() {
            let mut app = App::new();
            app.vaults = vec![Vault {
                id: "v1".to_string(),
                name: "Personal".to_string(),
            }];
            app.selected_vault_idx = None;

            assert!(app.selected_vault().is_none());
        }

        #[test]
        fn returns_none_when_index_out_of_bounds() {
            let mut app = App::new();
            app.vaults = vec![Vault {
                id: "v1".to_string(),
                name: "Personal".to_string(),
            }];
            app.selected_vault_idx = Some(5);

            assert!(app.selected_vault().is_none());
        }
    }
}
