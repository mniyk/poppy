use crate::commands::{self, Command};
use crate::provider::{Action, Candidate, Provider};

pub struct CommandProvider {
    commands: Vec<Command>,
}

impl CommandProvider {
    pub fn new() -> Self {
        Self {
            commands: commands::load(),
        }
    }
}

impl Provider for CommandProvider {
    fn name(&self) -> &'static str {
        "Command"
    }

    fn reload(&mut self) {
        self.commands = commands::load();
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }

        commands::search(&self.commands, q)
            .into_iter()
            .map(|c| Candidate {
                label: format!("{} を実行", c.name),
                source: "Command",
                action: Action::RunCommand {
                    command: c.command.clone(),
                    confirm: c.confirm,
                },
            })
            .collect()
    }
}
