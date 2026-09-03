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
        if q.is_empty() {
            return Vec::new();
        }

        bookmarks::search(&self.bookmarks, q)
            .into_iter()
            .map(|b| Candidate {
                label: format!("{} を開く", b.name),
                source: "Bookmark",
                action: Action::Open(b.url.clone()),
            })
            .collect()
    }
}
