use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub window: Window,
    pub providers: Providers,
    pub llm: Llm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    /// 呼び出しに使うグローバルホットキー
    pub hotkey: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Window {
    pub width: f64,
    pub height: f64,
    /// 背景の不透明度(0.0〜1.0)
    pub opacity: f64,
    /// 選択中の候補のハイライト色(#rrggbb)
    pub accent_color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Providers {
    pub window: bool,
    pub project: bool,
    pub bookmark: bool,
    pub snippet: bool,
    pub app: bool,
    pub websearch: bool,
    pub llm: bool,
    pub clipboard: bool,
    pub command: bool,
    pub todo: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Llm {
    /// Ollama サーバーのアドレス
    pub host: String,
    /// 使用するモデル名
    pub model: String,
    /// Tavily の API キー(空欄ならWeb検索なしでそのままLLMに質問する)
    pub tavily_api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General::default(),
            window: Window::default(),
            providers: Providers::default(),
            llm: Llm::default(),
        }
    }
}

impl Default for General {
    fn default() -> Self {
        Self {
            hotkey: "ctrl+alt+r".to_string(),
        }
    }
}

impl Default for Window {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            opacity: 0.85,
            accent_color: "#722f37".to_string(),
        }
    }
}

impl Default for Providers {
    fn default() -> Self {
        Self {
            window: true,
            project: true,
            bookmark: true,
            snippet: true,
            app: true,
            websearch: true,
            llm: true,
            clipboard: true,
            command: true,
            todo: true,
        }
    }
}

impl Default for Llm {
    fn default() -> Self {
        Self {
            host: "http://localhost:11434".to_string(),
            model: "gemma3:4b".to_string(),
            tavily_api_key: String::new(),
        }
    }
}

/// 設定ファイルのパスを返す
pub fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "poppy")?;
    Some(dirs.config_dir().join("config.toml"))
}

/// 設定を読み込む(無ければデフォルト)
pub fn load() -> Config {
    let Some(path) = config_path() else {
        return Config::default();
    };

    let Ok(text) = fs::read_to_string(&path) else {
        return Config::default();
    };

    match toml::from_str::<Config>(&text) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("config.toml の読み込みに失敗しました: {err}");
            Config::default()
        }
    }
}

/// 設定を保存する
pub fn save(config: &Config) -> Result<(), String> {
    let path = config_path().ok_or("設定ファイルのパスを取得できませんでした")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let text = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}
