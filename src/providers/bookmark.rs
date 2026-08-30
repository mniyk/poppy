use crate::bookmarks::{self, Bookmark};
use crate::provider::{Action, Candidate, Provider};

pub struct BookmarkProvider {
    bookmarks: Vec<Bookmark>,
}

impl BookmarkProvider {
    pub fn new() -> Self {
        Self {
            bookmarks: bookmarks::load(),
        }
    }
}

impl Provider for BookmarkProvider {
    fn name(&self) -> &'static str {
        "Bookmark"
    }

    fn reload(&mut self) {
        self.bookmarks = bookmarks::load();
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim();
        let mut list = Vec::new();

        // 設定ファイルを開くコマンド(空欄時は常に、入力時は名前が部分一致したら)
        let show_config = if q.is_empty() {
            true
        } else {
            let ql = q.to_lowercase();
            "open bookmarks".contains(&ql) || "config".contains(&ql)
        };

        if show_config {
            if let Some(path) = bookmarks::config_path() {
                list.push(Candidate {
                    label: "Open Bookmarks (設定ファイルを開く)".to_string(),
                    source: "Bookmark",
                    action: Action::Open(path.to_string_lossy().into_owned()),
                });
            }
        }

        if !q.is_empty() {
            for b in bookmarks::search(&self.bookmarks, q) {
                list.push(Candidate {
                    label: format!("{} を開く", b.name),
                    source: "Bookmark",
                    action: Action::Open(b.url.clone()),
                });
            }
        }

        list
    }
}
