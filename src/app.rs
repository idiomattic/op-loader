use ratatui::widgets::ListState;
use serde::Deserialize;
use std::{io, process::Command, time::Instant};

pub struct App {
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
}

impl App {
    pub fn new() -> Self {
        let app = Self {
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
        };

        app
    }

    fn run_op_command(&mut self, args: &[&str]) -> io::Result<Vec<u8>> {
        let cmd_str = format!("op {}", args.join(" "));

        let output = Command::new("op").args(args).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            self.command_log.log_failure(&cmd_str, &stderr);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("`{}` failed: {}", cmd_str, stderr),
            ));
        }

        Ok(output.stdout)
    }

    pub fn load_vaults(&mut self) -> io::Result<()> {
        let stdout = self.run_op_command(&["vault", "list", "--format", "json"])?;

        let vaults: Vec<Vault> = serde_json::from_slice(&stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

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

    pub fn load_accounts(&mut self) -> io::Result<()> {
        let stdout = self.run_op_command(&["account", "list", "--format", "json"])?;

        let accounts: Vec<Account> = serde_json::from_slice(&stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.command_log
            .log_success("op account list", Some(accounts.len()));

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

        let selected_vault_name = &self.selected_vault().unwrap().name.clone();

        let stdout = self.run_op_command(&[
            "item",
            "list",
            "--vault",
            selected_vault_name,
            "--format",
            "json",
        ])?;

        let vault_items: Vec<VaultItem> = serde_json::from_slice(&stdout)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.command_log.log_success(
            format!("op item list --vault {}", selected_vault_name),
            Some(vault_items.len()),
        );

        self.vault_items = vault_items;

        if !self.vault_items.is_empty() {
            self.vault_item_list_state.select(Some(0));
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct CommandLog {
    pub entries: Vec<CommandLogEntry>,
}

pub struct CommandLogEntry {
    pub command: String,
    pub status: CommandStatus,
    pub timestamp: Instant,
}

impl CommandLogEntry {
    pub fn display(&self) -> String {
        match &self.status {
            CommandStatus::Success { item_count } => match item_count {
                Some(n) => format!("✓ {} ({} items)", self.command, n),
                None => format!("✓ {}", self.command),
            },
            CommandStatus::Failed { stderr } => {
                let first_line = stderr.lines().next().unwrap_or("");
                format!("✗ {}: {}", self.command, first_line)
            }
        }
    }
}

pub enum CommandStatus {
    Success { item_count: Option<usize> },
    Failed { stderr: String },
}

impl CommandLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn log_success(&mut self, command: impl Into<String>, item_count: Option<usize>) {
        self.entries.push(CommandLogEntry {
            command: command.into(),
            status: CommandStatus::Success { item_count },
            timestamp: Instant::now(),
        });
        self.trim();
    }

    pub fn log_failure(&mut self, command: impl Into<String>, stderr: impl Into<String>) {
        self.entries.push(CommandLogEntry {
            command: command.into(),
            status: CommandStatus::Failed {
                stderr: stderr.into(),
            },
            timestamp: Instant::now(),
        });
        self.trim();
    }

    fn trim(&mut self) {
        const MAX_ENTRIES: usize = 50;
        if self.entries.len() > MAX_ENTRIES {
            self.entries.drain(0..self.entries.len() - MAX_ENTRIES);
        }
    }

    pub fn recent(&self, n: usize) -> &[CommandLogEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
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

#[derive(PartialEq)]
pub enum FocusedPanel {
    AccountList,
    VaultList,
    VaultItemList,
}
