use std::path::PathBuf;

use walkdir::WalkDir;

use crate::provider::{Action, Candidate, Provider};

/// インストール済みアプリ1件
#[derive(Debug, Clone)]
struct AppEntry {
    /// 表示名(ショートカットのファイル名から拡張子を除いたもの)
    name: String,
    /// .lnk のパス
    path: PathBuf,
}

pub struct AppLauncherProvider {
    apps: Vec<AppEntry>,
}

impl AppLauncherProvider {
    pub fn new() -> Self {
        Self { apps: scan_apps() }
    }
}

impl Provider for AppLauncherProvider {
    fn name(&self) -> &'static str {
        "App"
    }

    fn reload(&mut self) {
        self.apps = scan_apps();
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }

        self.apps
            .iter()
            .filter(|a| a.name.to_lowercase().contains(&q))
            .map(|a| Candidate {
                label: format!("{} を起動", a.name),
                source: "App",
                action: Action::Open(a.path.to_string_lossy().into_owned()),
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
                path: path.to_path_buf(),
            });
        }
    }

    // 同名のショートカットが重複しがちなので、名前で重複排除
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));

    apps
}
