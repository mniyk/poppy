use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Command {
    /// 候補に表示する名前
    pub name: String,
    /// 実行するコマンド(cmd.exe 経由で実行される)
    pub command: String,
    /// 実行前に確認ダイアログを出すか(破壊的な操作向け、省略時は false)
    #[serde(default)]
    pub confirm: bool,
    /// 検索にヒットさせる別名(省略可)
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct CommandFile {
    #[serde(default, rename = "command")]
    commands: Vec<Command>,
}

/// 設定ファイルのパスを返す
/// Windows: C:\Users\<user>\AppData\Roaming\poppy\config\commands.toml
/// Linux:   ~/.config/poppy/commands.toml
pub fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "poppy")?;
    Some(dirs.config_dir().join("commands.toml"))
}

/// コマンドを読み込む
/// ファイルが無ければ、サンプル付きの雛形を自動生成する
pub fn load() -> Vec<Command> {
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

    match toml::from_str::<CommandFile>(&text) {
        Ok(file) => file.commands,
        Err(err) => {
            eprintln!("commands.toml の読み込みに失敗しました: {err}");
            Vec::new()
        }
    }
}

/// 入力文字列にマッチするコマンドを返す
/// name か keywords のいずれかに部分一致すればヒット(大文字小文字は無視)
pub fn search(commands: &[Command], query: &str) -> Vec<Command> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    commands
        .iter()
        .filter(|c| {
            c.name.to_lowercase().contains(&q)
                || c.keywords.iter().any(|k| k.to_lowercase().contains(&q))
        })
        .cloned()
        .collect()
}

const DEFAULT_CONFIG: &str = r#"# Poppy のコマンド設定
# name: 候補に表示される名前
# command: 実行するコマンド(cmd.exe 経由で実行される)
# confirm: true にすると実行前に確認ダイアログを出す(破壊的な操作向け、省略時は false)
# keywords: 検索にヒットさせる別名(省略可)
#
# [[command]]
# name = "ごみ箱を空にする"
# command = "powershell -Command Clear-RecycleBin -Force"
# confirm = true
# keywords = ["trash", "recycle"]
"#;
