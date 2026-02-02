#[derive(Default)]
pub struct CommandLog {
    pub entries: Vec<CommandLogEntry>,
}

pub struct CommandLogEntry {
    pub command: String,
    pub status: CommandStatus,
}

pub enum CommandStatus {
    Success { item_count: Option<usize> },
    Failed { stderr: String },
}

impl CommandLogEntry {
    pub fn display(&self) -> String {
        match &self.status {
            CommandStatus::Success { item_count } => item_count.as_ref().map_or_else(
                || format!("✓ {}", self.command),
                |n| format!("✓ {} ({} items)", self.command, n),
            ),
            CommandStatus::Failed { stderr } => {
                let first_line = stderr.lines().next().unwrap_or("");
                format!("✗ {}: {}", self.command, first_line)
            }
        }
    }
}

impl CommandLog {
    pub fn log_success(&mut self, command: impl Into<String>, item_count: Option<usize>) {
        self.entries.push(CommandLogEntry {
            command: command.into(),
            status: CommandStatus::Success { item_count },
        });
        self.trim();
    }

    pub fn log_failure(&mut self, command: impl Into<String>, stderr: impl Into<String>) {
        self.entries.push(CommandLogEntry {
            command: command.into(),
            status: CommandStatus::Failed {
                stderr: stderr.into(),
            },
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
