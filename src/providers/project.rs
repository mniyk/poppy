use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::projects::{self, Project};
use crate::provider::{Action, Candidate, Provider};

pub struct ProjectProvider {
    projects: Vec<Project>,
}

impl ProjectProvider {
    pub fn new() -> Self {
        Self {
            projects: projects::load(),
        }
    }
}

impl Provider for ProjectProvider {
    fn name(&self) -> &'static str {
        "Project"
    }

    fn reload(&mut self) {
        self.projects = projects::load();
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }

        projects::search(&self.projects, q)
            .into_iter()
            .map(|p| Candidate {
                label: format!("{} を VSCode で開く", p.name),
                source: "Project",
                action: Action::OpenInVscode(p.path.clone()),
            })
            .collect()
    }
}

/// コンソールウィンドウを出さずにプロセスを起動するフラグ
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// VSCode で指定したパスを開く
pub fn open_in_vscode(path: &str) {
    // code.cmd は PATH 上にあるので cmd 経由で呼ぶ
    let result = Command::new("cmd")
        .args(["/C", "code", path])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    if let Err(err) = result {
        eprintln!("VSCode で開けませんでした ({path}): {err}");
    }
}

