use ratatui::widgets::{List, ListState};
use serde::Deserialize;
use std::{io, process::Command};

pub struct App {
    pub should_quit: bool,
    pub focused_panel: FocusedPanel,

    pub accounts: Vec<Account>,
    pub account_list_state: ListState,
    pub selected_account_idx: Option<usize>,

    pub vaults: Vec<Vault>,
    pub vault_list_state: ListState,
    pub selected_vault_idx: Option<usize>,

    pub vault_items: Vec<VaultItem>,
    pub vault_item_list_state: ListState,
    pub selected_vault_item_idx: Option<usize>,
}

impl App {
    pub fn new() -> Self {
        let app = Self {
            should_quit: false,
            focused_panel: FocusedPanel::VaultList,

            vaults: Vec::new(),
            vault_list_state: ListState::default(),
            selected_vault_idx: None,

            accounts: Vec::new(),
            account_list_state: ListState::default(),
            selected_account_idx: None,

            vault_items: Vec::new(),
            vault_item_list_state: ListState::default(),
            selected_vault_item_idx: None,
        };

        app
    }

    pub fn load_vaults(&mut self) -> io::Result<()> {
        let output = Command::new("op")
            .args(["vault", "list", "--format", "json"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("`op vault list` failed: {}", stderr),
            ));
        }

        let vaults: Vec<Vault> = serde_json::from_slice(&output.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.vaults = vaults;

        if !self.vaults.is_empty() {
            self.vault_list_state.select(Some(0));
        }

        Ok(())
    }

    pub fn load_accounts(&mut self) -> io::Result<()> {
        let output = Command::new("op")
            .args(["account", "list", "--format", "json"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stdout);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("`op account list` failed: {}", stderr),
            ));
        }

        let accounts: Vec<Account> = serde_json::from_slice(&output.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.accounts = accounts;

        if !self.accounts.is_empty() {
            self.account_list_state.select(Some(0));
        }

        Ok(())
    }

    pub fn load_vault_items(&mut self) -> io::Result<()> {
        if self.selected_account_idx.is_none() || self.selected_vault_idx.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("cannot list vault items when account/vault are not selected"),
            ));
        }

        let selected_vault_name = "";

        let output = Command::new("op")
            .args([
                "item",
                "list",
                "--vault",
                selected_vault_name,
                "--format",
                "json",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stdout);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("`op item list` failed: {}", stderr),
            ));
        }

        let vault_items: Vec<VaultItem> = serde_json::from_slice(&output.stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.vault_items = vault_items;

        if !self.vault_items.is_empty() {
            self.vault_item_list_state.select(Some(0));
        }

        Ok(())
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
struct ItemUrl {
    label: String,
    primary: bool,
    href: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultItem {
    pub id: String,
    pub title: String,
    pub category: String,
    pub additional_information: String,
    pub urls: Vec<ItemUrl>,
}

pub enum FocusedPanel {
    AccountList,
    VaultList,
    VaultItemList,
}
