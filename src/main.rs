mod app;
mod bookmarks;
mod commands;
mod config;
mod projects;
mod provider;
mod providers;
mod snippets;
mod todos;

use app::App;
use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;
use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};

/// アイコン画像(ビルド時に実行ファイルへ埋め込む)
const ICON_BYTES: &[u8] = include_bytes!("../assets/favicon.ico");

/// ICOを読み込んでウィンドウアイコンに変換する
fn window_icon() -> Option<Icon> {
    let img = image::load_from_memory(ICON_BYTES).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).ok()
}

fn main() {
    let mut window = WindowBuilder::new()
        .with_title("Poppy")
        .with_decorations(false)
        .with_always_on_top(true)
        .with_resizable(false)
        .with_undecorated_shadow(false)
        .with_transparent(true)
        .with_inner_size(LogicalSize::new(800.0, 600.0));

    if let Some(icon) = window_icon() {
        window = window.with_window_icon(Some(icon));
    }

    let config = Config::new()
        .with_window(window)
        .with_menu(None)
        .with_tray_icon_show_window_on_click(false);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}
