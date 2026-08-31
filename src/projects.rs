use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    /// 候補に表示する名前
    pub name: String,
    /// プロジェクトのパス
    pub path: String,
    /// 検索にヒットさせる別名(省略可)
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ProjectFile {
    #[serde(default, rename = "project")]
    projects: Vec<Project>,
}

/// 設定ファイルのパスを返す
pub fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "poppy")?;
    Some(dirs.config_dir().join("projects.toml"))
}

/// プロジェクト一覧を読み込む
/// ファイルが無ければ、サンプル付きの雛形を自動生成する
pub fn load() -> Vec<Project> {
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

    match toml::from_str::<ProjectFile>(&text) {
        Ok(file) => file.projects,
        Err(err) => {
            eprintln!("projects.toml の読み込みに失敗しました: {err}");
            Vec::new()
        }
    }
}

/// 入力文字列にマッチするプロジェクトを返す
pub fn search(projects: &[Project], query: &str) -> Vec<Project> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    projects
        .iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&q)
                || p.keywords.iter().any(|k| k.to_lowercase().contains(&q))
        })
        .cloned()
        .collect()
}

const DEFAULT_CONFIG: &str = r#"# Poppy のプロジェクト設定
# name: 候補に表示される名前
# path: プロジェクトのパス(バックスラッシュは2つ重ねる)
# keywords: 検索にヒットさせる別名(省略可)
#
# [[project]]
# name = "my-app"
# path = "C:\\Users\\you\\Work\\my-app"
# keywords = ["app"]
"#;
