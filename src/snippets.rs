use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Snippet {
    /// 候補に表示する名前
    pub name: String,
    /// 選択時にクリップボードへコピーする内容
    pub content: String,
    /// 検索にヒットさせる別名(省略可)
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SnippetFile {
    #[serde(default, rename = "snippet")]
    snippets: Vec<Snippet>,
}

/// 設定ファイルのパスを返す
/// Windows: C:\Users\<user>\AppData\Roaming\poppy\config\snippets.toml
/// Linux:   ~/.config/poppy/snippets.toml
pub fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "poppy")?;
    Some(dirs.config_dir().join("snippets.toml"))
}

/// スニペットを読み込む
/// ファイルが無ければ、サンプル付きの雛形を自動生成する
pub fn load() -> Vec<Snippet> {
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

    match toml::from_str::<SnippetFile>(&text) {
        Ok(file) => file.snippets,
        Err(err) => {
            eprintln!("snippets.toml の読み込みに失敗しました: {err}");
            Vec::new()
        }
    }
}

/// 入力文字列にマッチするスニペットを返す
/// name か keywords のいずれかに部分一致すればヒット(大文字小文字は無視)
pub fn search(snippets: &[Snippet], query: &str) -> Vec<Snippet> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    snippets
        .iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&q)
                || s.keywords.iter().any(|k| k.to_lowercase().contains(&q))
        })
        .cloned()
        .collect()
}

const DEFAULT_CONFIG: &str = r#"# Poppy のスニペット設定
# name: 候補に表示される名前
# content: 選択時にクリップボードへコピーされる内容
# keywords: 検索にヒットさせる別名(省略可)
#
# [[snippet]]
# name = "メール署名"
# content = "Yohei Kono"
# keywords = ["sig", "signature"]
"#;
