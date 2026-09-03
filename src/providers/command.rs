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
        let mut list = Vec::new();

        // 設定ファイルを開くコマンド(空欄時は常に、入力時は名前が部分一致したら)
        let show_config = if q.is_empty() {
            true
        } else {
            let ql = q.to_lowercase();
            "open commands".contains(&ql) || "config".contains(&ql)
        };

        if show_config {
            if let Some(path) = commands::config_path() {
                list.push(Candidate {
                    label: "Open Commands (設定ファイルを開く)".to_string(),
                    source: "Command",
                    action: Action::Open(path.to_string_lossy().into_owned()),
                });
            }
        }

        if !q.is_empty() {
            for c in commands::search(&self.commands, q) {
                list.push(Candidate {
                    label: format!("{} を実行", c.name),
                    source: "Command",
                    action: Action::RunCommand {
                        command: c.command.clone(),
                        confirm: c.confirm,
                    },
                });
            }
        }

        list
    }
}
