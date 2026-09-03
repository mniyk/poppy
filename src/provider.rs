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
    /// シェルコマンドを実行する(confirm が true なら実行前に確認ダイアログを出す)
    RunCommand { command: String, confirm: bool },
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
            Action::RunCommand { command, confirm } => {
                if *confirm && !confirm_dialog(command) {
                    return;
                }
                run_command(command);
            }
        }
    }
}

/// 「実行しますか?」の確認ダイアログを出し、Yes が選ばれたかを返す
fn confirm_dialog(command: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_ICONWARNING, MB_YESNO};

    let text = to_wide(&format!("次のコマンドを実行しますか?\n\n{command}"));
    let caption = to_wide("Poppy");

    let result = unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MB_YESNO | MB_ICONWARNING,
        )
    };

    result == IDYES
}

/// cmd.exe 経由でコマンドを実行する(結果を待たず、コンソールウィンドウも出さない)
fn run_command(command: &str) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let result = std::process::Command::new("cmd")
        .args(["/C", command])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    if let Err(err) = result {
        eprintln!("コマンドを実行できませんでした ({command}): {err}");
    }
}

fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
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
