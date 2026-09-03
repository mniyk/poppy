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
        if q.is_empty() {
            return Vec::new();
        }

        snippets::search(&self.snippets, q)
            .into_iter()
            .map(|s| Candidate {
                label: format!("{} をコピー", s.name),
                source: "Snippet",
                action: Action::CopyToClipboard(s.content.clone()),
            })
            .collect()
    }
}
