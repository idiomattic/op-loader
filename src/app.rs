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
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            should_quit: false,
            vaults: Vec::new(),
            vault_list_state: ListState::default(),
        };
        app
    }
}
