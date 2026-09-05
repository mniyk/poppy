use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use serde::Deserialize;
use walkdir::WalkDir;

use crate::provider::{Action, Candidate, Provider};

/// インストール済みアプリ1件
#[derive(Debug, Clone)]
pub struct AppEntry {
    /// 候補に表示する名前
    name: String,
    /// open::that() に渡す起動先(.lnk のパス、または shell:appsFolder\<AppID>)
    target: String,
}

/// MSIX(Microsoft Store)アプリの一覧。取得に時間がかかるため、
/// 起動時に一度だけバックグラウンドで取得してここにキャッシュする
pub type SharedStoreApps = Rc<RefCell<Vec<AppEntry>>>;

pub fn new_shared_store_apps() -> SharedStoreApps {
    Rc::new(RefCell::new(Vec::new()))
}

pub struct AppLauncherProvider {
    apps: Vec<AppEntry>,
    store_apps: SharedStoreApps,
}

impl AppLauncherProvider {
    pub fn new(store_apps: SharedStoreApps) -> Self {
        Self {
            apps: scan_apps(),
            store_apps,
        }
    }
}

impl Provider for AppLauncherProvider {
    fn name(&self) -> &'static str {
        "App"
    }

    fn reload(&mut self) {
        // .lnk のスキャンは高速なので毎回やり直す(MSIXアプリの一覧は再取得しない)
        self.apps = scan_apps();
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        self.apps
            .iter()
            .chain(self.store_apps.borrow().iter())
            .filter(|a| a.name.to_lowercase().contains(&q))
            .map(|a| Candidate {
                label: format!("{} を起動", a.name),
                source: "App",
                action: Action::Open(a.target.clone()),
            })
            .collect()
    }
}

/// スタートメニューを走査してショートカットを集める
fn scan_apps() -> Vec<AppEntry> {
    let mut roots: Vec<PathBuf> = Vec::new();

    // 全ユーザー共通のスタートメニュー
    if let Ok(program_data) = std::env::var("ProgramData") {
        roots.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    // ユーザー個別のスタートメニュー
    if let Ok(appdata) = std::env::var("APPDATA") {
        roots.push(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    let mut apps: Vec<AppEntry> = Vec::new();

    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let is_lnk = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lnk"))
                .unwrap_or(false);
            if !is_lnk {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            apps.push(AppEntry {
                name: name.to_string(),
                target: path.to_string_lossy().into_owned(),
            });
        }
    }

    // 同名のショートカットが重複しがちなので、名前で重複排除
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));

    apps
}

#[derive(Deserialize)]
struct StartApp {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AppID")]
    app_id: String,
}

/// Get-StartApps で MSIX(Microsoft Store)アプリを取得する
///
/// PowerShell の起動に数百ms かかることがあるため、これは起動時に一度だけ
/// バックグラウンドで呼び出す想定(ホットキーの reload() では呼ばない)
pub fn fetch_store_apps() -> Vec<AppEntry> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // PowerShell の標準出力は既定でシステムのANSIコードページ(日本語環境では
    // CP932)になり、UTF-8 として読むと失敗するため、明示的にUTF-8へ切り替える
    let result = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
             @(Get-StartApps) | ConvertTo-Json -Compress",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let Ok(output) = result else {
        return Vec::new();
    };
    let Ok(text) = String::from_utf8(output.stdout) else {
        return Vec::new();
    };
    let Ok(apps) = serde_json::from_str::<Vec<StartApp>>(&text) else {
        return Vec::new();
    };

    // AppID に "!" を含むものだけが MSIX アプリ。従来型アプリ(.lnk)は
    // scan_apps() 側で既にカバーしているので、ここでは除外して重複を避ける
    apps.into_iter()
        .filter(|a| a.app_id.contains('!'))
        .map(|a| AppEntry {
            name: a.name,
            target: format!("shell:appsFolder\\{}", a.app_id),
        })
        .collect()
}
