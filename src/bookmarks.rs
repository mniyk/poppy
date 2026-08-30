use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Bookmark {
    /// 候補に表示する名前
    pub name: String,
    /// 開くURL
    pub url: String,
    /// 検索にヒットさせる別名(省略可)
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct BookmarkFile {
    #[serde(default, rename = "bookmark")]
    bookmarks: Vec<Bookmark>,
}

/// 設定ファイルのパスを返す
/// Windows: C:\Users\<user>\AppData\Roaming\poppy\config\bookmarks.toml
/// Linux:   ~/.config/poppy/bookmarks.toml
pub fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "poppy")?;
    Some(dirs.config_dir().join("bookmarks.toml"))
}

/// ブックマークを読み込む
/// ファイルが無ければ、サンプル付きの雛形を自動生成する
pub fn load() -> Vec<Bookmark> {
    let Some(path) = config_path() else {
        return Vec::new();
    };

    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path.as_path(), DEFAULT_CONFIG);
    }

    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };

    match toml::from_str::<BookmarkFile>(&text) {
        Ok(file) => file.bookmarks,
        Err(err) => {
            eprintln!("bookmarks.toml の読み込みに失敗しました: {err}");
            Vec::new()
        }
    }
}

/// 入力文字列にマッチするブックマークを返す
/// name か keywords のいずれかに部分一致すればヒット(大文字小文字は無視)
pub fn search(bookmarks: &[Bookmark], query: &str) -> Vec<Bookmark> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    bookmarks
        .iter()
        .filter(|b| {
            b.name.to_lowercase().contains(&q)
                || b.keywords.iter().any(|k| k.to_lowercase().contains(&q))
        })
        .cloned()
        .collect()
}

const DEFAULT_CONFIG: &str = r#"# Poppy のブックマーク設定
# name: 候補に表示される名前
# url: 開くURL
# keywords: 検索にヒットさせる別名(省略可)

[[bookmark]]
name = "GitHub"
url = "https://github.com"
keywords = ["gh"]

[[bookmark]]
name = "Google Drive"
url = "https://drive.google.com"
keywords = ["drive", "gdrive"]
"#;
