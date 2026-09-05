use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Todo {
    pub id: u64,
    pub text: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TodoFile {
    #[serde(default, rename = "todo")]
    todos: Vec<Todo>,
}

/// アプリ内で共有する TODO の状態(追加・完了のたびに書き換える)
pub type SharedTodos = Rc<RefCell<Vec<Todo>>>;

/// 起動時に一度だけ読み込み、共有状態として返す
pub fn new_shared() -> SharedTodos {
    Rc::new(RefCell::new(load()))
}

/// 設定ファイルのパスを返す
/// Windows: C:\Users\<user>\AppData\Roaming\poppy\config\todos.toml
/// Linux:   ~/.config/poppy/todos.toml
pub fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "poppy")?;
    Some(dirs.config_dir().join("todos.toml"))
}

/// TODO を読み込む(ファイルが無ければ空のリストを返す)
pub fn load() -> Vec<Todo> {
    let Some(path) = config_path() else {
        return Vec::new();
    };

    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };

    match toml::from_str::<TodoFile>(&text) {
        Ok(file) => file.todos,
        Err(err) => {
            eprintln!("todos.toml の読み込みに失敗しました: {err}");
            Vec::new()
        }
    }
}

/// TODO を保存する(Poppy 自身が追加・完了のたびに呼び出す)
pub fn save(todos: &[Todo]) -> Result<(), String> {
    let path = config_path().ok_or("設定ファイルのパスを取得できませんでした")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = TodoFile {
        todos: todos.to_vec(),
    };
    let text = toml::to_string_pretty(&file).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}

/// 新しい TODO に割り当てる ID(既存の最大値 + 1)
pub fn next_id(todos: &[Todo]) -> u64 {
    todos.iter().map(|t| t.id).max().unwrap_or(0) + 1
}
