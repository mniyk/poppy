use crate::provider::{Action, Candidate, Provider};
use crate::snippets::{self, Snippet};

pub struct SnippetProvider {
    snippets: Vec<Snippet>,
}

impl SnippetProvider {
    pub fn new() -> Self {
        Self {
            snippets: snippets::load(),
        }
    }
}

impl Provider for SnippetProvider {
    fn name(&self) -> &'static str {
        "Snippet"
    }

    fn reload(&mut self) {
        self.snippets = snippets::load();
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim();
        let mut list = Vec::new();

        // 設定ファイルを開くコマンド(空欄時は常に、入力時は名前が部分一致したら)
        let show_config = if q.is_empty() {
            true
        } else {
            let ql = q.to_lowercase();
            "open snippets".contains(&ql) || "config".contains(&ql)
        };

        if show_config {
            if let Some(path) = snippets::config_path() {
                list.push(Candidate {
                    label: "Open Snippets (設定ファイルを開く)".to_string(),
                    source: "Snippet",
                    action: Action::Open(path.to_string_lossy().into_owned()),
                });
            }
        }

        if !q.is_empty() {
            for s in snippets::search(&self.snippets, q) {
                list.push(Candidate {
                    label: format!("{} をコピー", s.name),
                    source: "Snippet",
                    action: Action::CopyToClipboard(s.content.clone()),
                });
            }
        }

        list
    }
}
