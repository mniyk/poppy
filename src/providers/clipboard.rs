use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use crate::provider::{Action, Candidate, Provider};

/// 保持するクリップボード履歴の最大件数
const MAX_HISTORY: usize = 30;
/// 候補に表示する文字数の上限
const MAX_LABEL_CHARS: usize = 60;
/// クリップボードを確認する間隔
pub const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub type ClipboardHistory = Rc<RefCell<VecDeque<String>>>;

pub fn new_history() -> ClipboardHistory {
    Rc::new(RefCell::new(VecDeque::new()))
}

/// クリップボードの中身を確認し、前回と違っていれば履歴の先頭に追加する
///
/// `last_seen` は直前に確認した内容を保持しておくための状態で、ポーリングの
/// たびに OS のクリップボードへ問い合わせるコストを避けるためのものではなく、
/// 同じ内容を重複して履歴の先頭に積み直さないようにするために使う
///
/// 戻り値は履歴を実際に更新したかどうか。履歴は Signal ではない共有データなので、
/// 呼び出し側はこれを見て、候補一覧の再計算をトリガーする必要がある
pub fn poll(history: &ClipboardHistory, last_seen: &mut Option<String>) -> bool {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        return false;
    };
    let Ok(text) = clipboard.get_text() else {
        return false;
    };

    let text = text.trim().to_string();
    if text.is_empty() || last_seen.as_deref() == Some(text.as_str()) {
        return false;
    }
    *last_seen = Some(text.clone());

    let mut list = history.borrow_mut();
    list.retain(|existing| existing != &text);
    list.push_front(text);
    list.truncate(MAX_HISTORY);
    true
}

/// クリップボード履歴から候補を出すプロバイダ
pub struct ClipboardProvider {
    history: ClipboardHistory,
}

impl ClipboardProvider {
    pub fn new(history: ClipboardHistory) -> Self {
        Self { history }
    }
}

impl Provider for ClipboardProvider {
    fn name(&self) -> &'static str {
        "Clipboard"
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim().to_lowercase();

        self.history
            .borrow()
            .iter()
            .filter(|text| q.is_empty() || text.to_lowercase().contains(&q))
            .map(|text| Candidate {
                label: format!("「{}」をコピー", truncate(text)),
                source: "Clipboard",
                action: Action::CopyToClipboard(text.clone()),
            })
            .collect()
    }
}

/// 候補表示用に、改行をつぶして長さを切り詰める
fn truncate(text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let char_count = collapsed.chars().count();
    if char_count <= MAX_LABEL_CHARS {
        return collapsed;
    }

    let truncated: String = collapsed.chars().take(MAX_LABEL_CHARS).collect();
    format!("{truncated}…")
}
