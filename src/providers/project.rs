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
        let mut list = Vec::new();

        // 設定ファイルを開くコマンド(空欄時は常に、入力時は名前が部分一致したら)
        let show_config = if q.is_empty() {
            true
        } else {
            let ql = q.to_lowercase();
            "open projects".contains(&ql)
        };

        if show_config {
            if let Some(path) = projects::config_path() {
                list.push(Candidate {
                    label: "Open Projects (設定ファイルを開く)".to_string(),
                    source: "Project",
                    action: Action::Open(path.to_string_lossy().into_owned()),
                });
            }
        }

        if !q.is_empty() {
            for p in projects::search(&self.projects, q) {
                list.push(Candidate {
                    label: format!("{} を VSCode で開く", p.name),
                    source: "Project",
                    action: Action::OpenInVscode(p.path.clone()),
                });
            }
        }

        list
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

