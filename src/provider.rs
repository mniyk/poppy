/// 候補を実行したときの動作
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// URLまたはファイルパスを既定のアプリで開く
    Open(String),
    /// 指定したウィンドウを前面に出す
    FocusWindow(isize),
    /// VSCode で指定したパスを開く
    OpenInVscode(String),
    /// 文字列をクリップボードにコピーする
    CopyToClipboard(String),
    /// AIに質問する(非同期処理が必要なため、UI側でEnter入力時に
    /// 特別扱いして実行する。run() からは呼ばれない)
    AskLlm(String),
}

impl Action {
    pub fn run(&self) {
        match self {
            Action::Open(target) => {
                if let Err(err) = open::that(target) {
                    eprintln!("開けませんでした ({target}): {err}");
                }
            }
            Action::FocusWindow(hwnd) => {
                crate::providers::window::focus(*hwnd);
            }
            Action::OpenInVscode(path) => {
                crate::providers::project::open_in_vscode(path);
            }
            Action::CopyToClipboard(text) => {
                // WebView2 のカスタムアセットオリジンはセキュアコンテキストと見なされず
                // navigator.clipboard が使えないため、OS のクリップボードに直接書き込む
                match arboard::Clipboard::new().and_then(|mut c| c.set_text(text.clone())) {
                    Ok(()) => {}
                    Err(err) => eprintln!("クリップボードにコピーできませんでした: {err}"),
                }
            }
            Action::AskLlm(_) => {}
        }
    }
}

/// 候補1件分
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// 候補に表示する文字列
    pub label: String,
    /// 提供元のプロバイダ名
    pub source: &'static str,
    /// 実行時の動作
    pub action: Action,
}

/// 候補を提供するプロバイダ
pub trait Provider {
    /// プロバイダ名
    fn name(&self) -> &'static str;

    /// 設定の再読み込み(必要なプロバイダだけ実装すればよい)
    fn reload(&mut self) {}

    /// 入力文字列に対する候補を返す
    fn candidates(&self, query: &str) -> Vec<Candidate>;
}
