use crate::provider::{Action, Candidate, Provider};

/// 検索エンジン1つ分の定義
struct Engine {
    name: &'static str,
    url_template: &'static str,
}

const ENGINES: &[Engine] = &[
    Engine {
        name: "Google",
        url_template: "https://www.google.com/search?q={}",
    },
    Engine {
        name: "DuckDuckGo",
        url_template: "https://duckduckgo.com/?q={}",
    },
];

pub struct WebSearchProvider;

impl Provider for WebSearchProvider {
    fn name(&self) -> &'static str {
        "Web Search"
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }

        let encoded = urlencoding::encode(q).into_owned();
        ENGINES
            .iter()
            .map(|e| Candidate {
                label: format!("{} で「{q}」を検索", e.name),
                source: "Web Search",
                action: Action::Open(e.url_template.replace("{}", &encoded)),
            })
            .collect()
    }
}