use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
pub struct Vault {
    pub id: String,
    pub name: String,
}

pub struct App {
    pub should_quit: bool,

    pub vaults: Vec<Vault>,

    pub vault_list_state: ListState,
    pub selected_vault_idx: Option<usize>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            should_quit: false,
            vaults: Vec::new(),
            vault_list_state: ListState::default(),
            selected_vault_idx: None,
        };

        app.vaults = vec![
            Vault {
                id: "vault1".to_string(),
                name: "Personal".to_string(),
            },
            Vault {
                id: "vault2".to_string(),
                name: "Work".to_string(),
            },
            Vault {
                id: "vault3".to_string(),
                name: "Shared".to_string(),
            },
        ];

        app.vault_list_state.select(Some(0));

        app
    }
}
