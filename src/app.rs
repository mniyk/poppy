use dioxus::desktop::tao::dpi::PhysicalPosition;
use dioxus::desktop::trayicon::{
    init_tray_icon,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    MouseButton, TrayIconEvent,
};
use dioxus::desktop::{
    use_global_shortcut, use_muda_event_handler, use_tray_icon_event_handler,
    use_tray_menu_event_handler, use_window, HotKeyState,
};
use dioxus::document;
use dioxus::prelude::*;

use crate::provider::{Candidate, Provider};
use crate::providers::{
    app_launcher::AppLauncherProvider, bookmark::BookmarkProvider, project::ProjectProvider,
    websearch::WebSearchProvider, window::WindowProvider,
};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[component]
pub fn App() -> Element {
    let window = use_window();
    let mut query = use_signal(String::new);
    let mut selected = use_signal(|| 0usize);

    // プロバイダ一覧(この順に候補が並ぶ)
    let mut providers = use_signal(|| {
        let list: Vec<Box<dyn Provider>> = vec![
            Box::new(WindowProvider::new()),
            Box::new(ProjectProvider::new()),
            Box::new(BookmarkProvider::new()),
            Box::new(AppLauncherProvider::new()),
            Box::new(WebSearchProvider),
        ];
        list
    });

    // 入力に応じた候補一覧
    let candidates: Vec<Candidate> = {
        let q = query.read().clone();
        providers
            .read()
            .iter()
            .flat_map(|p| p.candidates(&q))
            .collect()
    };

    // 選択が変わったら、その候補が見えるようにスクロールする
    use_effect(move || {
        let _ = selected();
        spawn(async move {
            let _ = document::eval(
                r#"
                const el = document.getElementById('selected-item');
                if (el) { el.scrollIntoView({ block: 'nearest' }); }
                "#,
            )
            .await;
        });
    });

    // ウィンドウを画面中央に配置
    use_effect({
        let window = window.clone();
        move || {
            if let Some(monitor) = window.current_monitor() {
                let monitor_size = monitor.size();
                let window_size = window.outer_size();
                let x = (monitor_size.width as i32 - window_size.width as i32) / 2;
                let y = (monitor_size.height as i32 - window_size.height as i32) / 2;
                window.set_outer_position(PhysicalPosition::new(x, y));
            }
        }
    });

    // グローバルホットキー(Ctrl+Alt+R)で表示/非表示をトグル
    let _ = use_global_shortcut("ctrl+alt+r", {
        let window = window.clone();
        move |state| {
            if state == HotKeyState::Pressed {
                if window.is_visible() {
                    window.set_visible(false);
                } else {
                    // 表示のたびに各プロバイダの設定を読み直す
                    providers.write().iter_mut().for_each(|p| p.reload());
                    query.set(String::new());
                    selected.set(0);
                    window.set_visible(true);
                    window.set_focus();
                }
            }
        }
    });

    // タスクトレイの初期化(初回マウント時に1回だけ)
    use_hook(|| {
        let tray_menu = Menu::new();
        let quit_item = MenuItem::with_id("quit", "Quit", true, None);
        let _ = tray_menu.append_items(&[&PredefinedMenuItem::separator(), &quit_item]);

        // アイコン画像を読み込む
        let icon = image::load_from_memory(include_bytes!("../assets/favicon.ico"))
            .ok()
            .map(|img| img.into_rgba8())
            .and_then(|img| {
                let (w, h) = img.dimensions();
                dioxus::desktop::trayicon::Icon::from_rgba(img.into_raw(), w, h).ok()
            });

        let tray_icon = init_tray_icon(tray_menu, icon);
        Box::leak(Box::new(tray_icon));
    });

    // ウィンドウの外にフォーカスが移ったら隠す(webview内のblurイベントで検知)
    use_future({
        let window = window.clone();
        move || {
            let window = window.clone();
            async move {
                let mut eval = document::eval(
                    r#"window.addEventListener('blur', () => { dioxus.send('blur'); });"#,
                );
                while eval.recv::<String>().await.is_ok() {
                    window.set_visible(false);
                }
            }
        }
    });

    // タスクトレイのアイコン左クリックでウィンドウを表示
    use_tray_icon_event_handler({
        let window = window.clone();
        move |event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                providers.write().iter_mut().for_each(|p| p.reload());
                query.set(String::new());
                selected.set(0);
                window.set_visible(true);
                window.set_focus();
            }
        }
    });

    // トレイメニューの「Quit」: 2系統の両方を監視
    use_tray_menu_event_handler(move |event| {
        if event.id.0 == "quit" {
            std::process::exit(0);
        }
    });
    use_muda_event_handler(move |event| {
        if event.id.0 == "quit" {
            std::process::exit(0);
        }
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div {
            class: "w-full h-full flex flex-col text-neutral-100 box-border overflow-hidden px-4 py-2 rounded-xl",
            style: "background-color: rgba(0, 0, 0, 0.85);",
            onkeydown: {
                let window = window.clone();
                move |e: Event<KeyboardData>| {
                    if e.key() == Key::Escape {
                        window.set_visible(false);
                    }
                }
            },

            // 入力欄
            input {
                class: "w-full px-6 py-5 text-2xl bg-transparent outline-none border-0 border-b border-neutral-700 placeholder:text-neutral-500",
                placeholder: "検索…",
                value: "{query}",
                oninput: move |e| {
                    query.set(e.value());
                    selected.set(0);
                },
                onmounted: move |e| {
                    spawn(async move {
                        let _ = e.set_focus(true).await;
                    });
                },
                onkeydown: {
                    let window = window.clone();
                    let candidates = candidates.clone();
                    move |e: Event<KeyboardData>| {
                        let len = candidates.len();
                        if len == 0 {
                            return;
                        }
                        match e.key() {
                            Key::ArrowDown => {
                                e.prevent_default();
                                selected.set((selected() + 1) % len);
                            }
                            Key::ArrowUp => {
                                e.prevent_default();
                                selected.set((selected() + len - 1) % len);
                            }
                            Key::Enter => {
                                if let Some(c) = candidates.get(selected()) {
                                    c.action.run();
                                    query.set(String::new());
                                    selected.set(0);
                                    window.set_visible(false);
                                }
                            }
                            _ => {}
                        }
                    }
                },
            }

            // 候補リスト
            div {
                class: "flex-1 overflow-y-auto py-2",
                if candidates.is_empty() {
                    div { class: "px-6 py-3 text-neutral-500", "コマンドを入力してください" }
                } else {
                    for (i, c) in candidates.iter().enumerate() {
                        div {
                            key: "{i}-{c.label}",
                            id: if selected() == i { "selected-item" } else { "" },
                            class: if selected() == i {
                                "px-6 py-3 bg-neutral-800 text-neutral-100 rounded"
                            } else {
                                "px-6 py-3 text-neutral-400 rounded"
                            },
                            "{c.label}"
                        }
                    }
                }
            }
        }
    }
}
