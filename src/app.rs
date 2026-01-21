use ratatui::widgets::ListState;
use serde::Deserialize;
use std::{io, process::Command};

pub struct App {
    pub should_quit: bool,
    pub focused_panel: FocusedPanel,

    pub vaults: Vec<Vault>,

    pub vault_list_state: ListState,
    pub selected_vault_idx: Option<usize>,
}

impl App {
    pub fn new() -> Self {
        let app = Self {
            should_quit: false,
            focused_panel: FocusedPanel::VaultList,
            vaults: Vec::new(),
            vault_list_state: ListState::default(),
            selected_vault_idx: None,
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
                format!("op vault list failed: {}", stderr),
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct Vault {
    pub id: String,
    pub name: String,
}

pub enum FocusedPanel {
    VaultList,
}
