use dioxus::desktop::tao::dpi::PhysicalPosition;
use dioxus::desktop::tao::event::{Event as WryEvent, WindowEvent};
use dioxus::desktop::trayicon::{
    init_tray_icon,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    MouseButton, TrayIconEvent,
};
use dioxus::desktop::{
    use_global_shortcut, use_muda_event_handler, use_tray_icon_event_handler,
    use_tray_menu_event_handler, use_window, use_wry_event_handler, HotKeyState,
};
use dioxus::document;
use dioxus::prelude::*;

use crate::config;
use crate::provider::{Action, Candidate, Provider};
use crate::providers::{
    app_launcher::AppLauncherProvider, bookmark::BookmarkProvider, clipboard,
    clipboard::ClipboardProvider, command::CommandProvider, llm::LlmProvider,
    project::ProjectProvider, snippet::SnippetProvider, websearch::WebSearchProvider,
    window::WindowProvider,
};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

/// 表示中の画面
#[derive(Clone, Copy, PartialEq)]
enum View {
    Search,
    Settings,
    LlmAnswer,
}

/// AI への質問の状態
#[derive(Clone, PartialEq)]
enum LlmAnswerState {
    Idle,
    /// 生成中(partial はここまでに届いた分の回答)
    Loading {
        question: String,
        partial: String,
    },
    Done {
        question: String,
        answer: String,
    },
    Error {
        question: String,
        message: String,
    },
}

#[component]
pub fn App() -> Element {
    let window = use_window();
    let mut query = use_signal(String::new);
    let mut selected = use_signal(|| 0usize);
    let mut view = use_signal(|| View::Search);
    let mut llm_answer = use_signal(|| LlmAnswerState::Idle);
    let cfg = use_signal(config::load);

    // クリップボード履歴(ウィンドウが隠れている間もバックグラウンドで蓄積する)
    let clipboard_history = use_hook(clipboard::new_history);
    use_hook({
        let clipboard_history = clipboard_history.clone();
        move || {
            spawn(async move {
                let mut last_seen = None;
                loop {
                    clipboard::poll(&clipboard_history, &mut last_seen);
                    tokio::time::sleep(clipboard::POLL_INTERVAL).await;
                }
            });
        }
    });

    // プロバイダ一覧(この順に候補が並ぶ。AIへの質問は常に先頭に出す)
    let mut providers = use_signal(|| {
        let list: Vec<Box<dyn Provider>> = vec![
            Box::new(LlmProvider),
            Box::new(ClipboardProvider::new(clipboard_history.clone())),
            Box::new(WindowProvider::new()),
            Box::new(ProjectProvider::new()),
            Box::new(BookmarkProvider::new()),
            Box::new(SnippetProvider::new()),
            Box::new(CommandProvider::new()),
            Box::new(AppLauncherProvider::new()),
            Box::new(WebSearchProvider),
        ];
        list
    });

    // 描画に使う設定値を先に取り出す(read のガードを rsx! に持ち込まない)
    let current = cfg.read().clone();
    let opacity = current.window.opacity;
    let hotkey = current.general.hotkey.clone();
    let llm_host = current.llm.host.clone();
    let llm_model = current.llm.model.clone();

    // AI への質問を実行するコールバック。App のスコープに所有されるので、
    // 呼び出し後に SearchView -> LlmAnswerView へ画面が切り替わって
    // SearchView がアンマウントされても、spawn したタスクは破棄されずに続行する
    // (spawn は「呼び出したスコープ」ではなく「コールバックの生成元スコープ」に紐づく)
    let ask_llm = use_callback(move |prompt: String| {
        let host = llm_host.clone();
        let model = llm_model.clone();
        let question = prompt.clone();
        llm_answer.set(LlmAnswerState::Loading {
            question: question.clone(),
            partial: String::new(),
        });
        view.set(View::LlmAnswer);
        spawn(async move {
            let question_for_chunks = question.clone();
            let result = crate::providers::llm::ask(&host, &model, &prompt, move |partial| {
                llm_answer.set(LlmAnswerState::Loading {
                    question: question_for_chunks.clone(),
                    partial: partial.to_string(),
                });
            })
            .await;
            llm_answer.set(match result {
                Ok(answer) => LlmAnswerState::Done { question, answer },
                Err(message) => LlmAnswerState::Error { question, message },
            });
        });
    });

    let enabled = [
        current.providers.llm,
        current.providers.clipboard,
        current.providers.window,
        current.providers.project,
        current.providers.bookmark,
        current.providers.snippet,
        current.providers.command,
        current.providers.app,
        current.providers.websearch,
    ];

    // 入力に応じた候補一覧(有効なプロバイダのみ)
    let candidates: Vec<Candidate> = {
        let q = query.read().clone();
        providers
            .read()
            .iter()
            .enumerate()
            .filter(|(i, _)| enabled.get(*i).copied().unwrap_or(true))
            .flat_map(|(_, provider)| provider.candidates(&q))
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

    // グローバルホットキーで表示/非表示をトグル
    let _ = use_global_shortcut(hotkey.as_str(), {
        let window = window.clone();
        move |state| {
            if state == HotKeyState::Pressed {
                if window.is_visible() {
                    window.set_visible(false);
                } else {
                    providers.write().iter_mut().for_each(|p| p.reload());
                    query.set(String::new());
                    selected.set(0);
                    view.set(View::Search);
                    llm_answer.set(LlmAnswerState::Idle);
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

    // ウィンドウの外にフォーカスが移ったら隠す
    // (JS の window blur イベントは WebView2 内部でのクリック時にも誤発火するため、
    //  OS レベルのネイティブなフォーカスイベントを使う。
    //  非表示→表示の直後は、フォーカスが親ウィンドウと WebView2 の子要素の間を
    //  行き来する影響で Focused イベントが true/false と短時間に何度も飛んでくることが
    //  あるため、イベント単体では判断せず、少し待ってから実際のフォーカス状態を
    //  window.is_focused() で確認してから隠す)
    use_wry_event_handler({
        let window = window.clone();
        move |event, _target| {
            if let WryEvent::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } = event
            {
                if !focused && !matches!(llm_answer(), LlmAnswerState::Loading { .. }) {
                    let window = window.clone();
                    spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        if !window.is_focused() {
                            window.set_visible(false);
                        }
                    });
                }
            }
        }
    });

    // Esc: 設定画面なら検索に戻る、検索ならウィンドウを隠す
    // (フォーカス中の要素からのイベント伝播に頼らず、document レベルで確実に拾う)
    use_future({
        let window = window.clone();
        move || {
            let window = window.clone();
            async move {
                let mut eval = document::eval(
                    r#"document.addEventListener('keydown', (e) => {
                        if (e.key === 'Escape') { dioxus.send('escape'); }
                    });"#,
                );
                while eval.recv::<String>().await.is_ok() {
                    if view() == View::LlmAnswer
                        && matches!(llm_answer(), LlmAnswerState::Loading { .. })
                    {
                        // 回答待ちの間は Esc で消えないようにする
                    } else if view() == View::Settings || view() == View::LlmAnswer {
                        view.set(View::Search);
                    } else {
                        window.set_visible(false);
                    }
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
                view.set(View::Search);
                llm_answer.set(LlmAnswerState::Idle);
                window.set_visible(true);
                window.set_focus();
            }
        }
    });

    // トレイメニューの「Quit」
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

    let bg = format!("background-color: rgba(0, 0, 0, {opacity});");
    let current_view = view();

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        div {
            class: "w-full h-full flex flex-col text-neutral-100 box-border overflow-hidden px-4 py-2 rounded-xl",
            style: "{bg}",

            match current_view {
                View::Search => rsx! {
                    SearchView {
                        query,
                        selected,
                        view,
                        candidates: candidates.clone(),
                        ask_llm,
                    }
                },
                View::Settings => rsx! {
                    SettingsView {
                        cfg,
                        view,
                        clipboard_history: clipboard_history.clone(),
                    }
                },
                View::LlmAnswer => rsx! {
                    LlmAnswerView { llm_answer, view }
                },
            }
        }
    }
}

/// 検索画面
#[component]
fn SearchView(
    query: Signal<String>,
    selected: Signal<usize>,
    view: Signal<View>,
    candidates: Vec<Candidate>,
    ask_llm: Callback<String>,
) -> Element {
    let window = use_window();
    let mut query = query;
    let mut selected = selected;
    let mut view = view;

    let current_query = query.read().clone();
    let current_selected = selected();

    rsx! {
        // 入力欄
        input {
            class: "w-full px-6 py-5 text-2xl bg-transparent outline-none border-0 placeholder:text-neutral-500",
            placeholder: "検索…",
            value: "{current_query}",
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
                let candidates = candidates.clone();
                move |e: Event<KeyboardData>| {
                    // 設定画面を開く
                    if e.key() == Key::Character(",".to_string()) && e.modifiers().ctrl() {
                        e.prevent_default();
                        view.set(View::Settings);
                        return;
                    }

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
                                if let Action::AskLlm(prompt) = c.action.clone() {
                                    ask_llm.call(prompt);
                                    query.set(String::new());
                                    selected.set(0);
                                } else {
                                    c.action.run();
                                    query.set(String::new());
                                    selected.set(0);
                                    window.set_visible(false);
                                }
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
                        id: if current_selected == i { "selected-item" } else { "" },
                        class: if current_selected == i {
                            "px-6 py-3 bg-neutral-800 text-neutral-100 rounded truncate"
                        } else {
                            "px-6 py-3 text-neutral-400 rounded truncate"
                        },
                        "{c.label}"
                    }
                }
            }
        }

        // フッター
        div {
            class: "px-6 py-2 text-xs text-neutral-600 text-right",
            "Ctrl+, で設定"
        }
    }
}

/// AIの回答画面
#[component]
fn LlmAnswerView(llm_answer: Signal<LlmAnswerState>, view: Signal<View>) -> Element {
    let mut view = view;
    let state = llm_answer.read().clone();
    let question = match &state {
        LlmAnswerState::Idle => "",
        LlmAnswerState::Loading { question, .. } => question,
        LlmAnswerState::Done { question, .. } => question,
        LlmAnswerState::Error { question, .. } => question,
    };

    rsx! {
        div {
            class: "flex items-center justify-between px-6 py-5",
            h2 { class: "text-lg text-neutral-300 truncate", "「{question}」" }
            button {
                class: "px-3 py-1 text-sm text-neutral-400 hover:text-neutral-100",
                onclick: move |_| view.set(View::Search),
                "戻る (Esc)"
            }
        }

        div {
            class: "flex-1 overflow-y-auto px-6 py-4 whitespace-pre-wrap",
            match &state {
                LlmAnswerState::Idle => rsx! {},
                LlmAnswerState::Loading { partial, .. } => rsx! {
                    if partial.is_empty() {
                        div { class: "text-neutral-500", "考え中…" }
                    } else {
                        div { "{partial}" }
                    }
                },
                LlmAnswerState::Done { answer, .. } => rsx! {
                    div { "{answer}" }
                },
                LlmAnswerState::Error { message, .. } => rsx! {
                    div { class: "text-red-400", "{message}" }
                },
            }
        }
    }
}

/// 設定画面
#[component]
fn SettingsView(
    cfg: Signal<config::Config>,
    view: Signal<View>,
    clipboard_history: clipboard::ClipboardHistory,
) -> Element {
    let mut cfg = cfg;
    let mut view = view;
    let mut message = use_signal(String::new);
    let clipboard_count = clipboard_history.borrow().len();

    // 描画に使う値を先に取り出す(read のガードを rsx! に持ち込まない)
    let current = cfg.read().clone();
    let hotkey = current.general.hotkey.clone();
    let width = current.window.width;
    let height = current.window.height;
    let opacity = current.window.opacity;
    let opacity_pct = ((opacity - 0.3) / (1.0 - 0.3) * 100.0).clamp(0.0, 100.0);
    let p = current.providers.clone();
    let llm_host = current.llm.host.clone();
    let llm_model = current.llm.model.clone();
    let current_message = message.read().clone();

    rsx! {
        div {
            class: "flex items-center justify-between px-6 py-5",
            h2 { class: "text-2xl", "設定" }
            button {
                class: "px-3 py-1 text-sm text-neutral-400 hover:text-neutral-100",
                onclick: move |_| view.set(View::Search),
                "閉じる (Esc)"
            }
        }

        div {
            class: "flex-1 overflow-y-auto px-6 py-4 space-y-6",

            // ホットキー
            div {
                label { class: "block text-sm text-neutral-400 mb-1", "ホットキー" }
                input {
                    class: "w-full px-3 py-2 bg-neutral-800 rounded outline-none border border-neutral-700 focus:border-neutral-500",
                    value: "{hotkey}",
                    oninput: move |e| cfg.write().general.hotkey = e.value(),
                    onmounted: move |e| {
                        spawn(async move {
                            let _ = e.set_focus(true).await;
                        });
                    },
                }
                p { class: "mt-1 text-xs text-neutral-600", "例: ctrl+alt+r  (変更後は再起動が必要)" }
            }

            // ウィンドウサイズ
            div {
                class: "flex gap-4",
                div {
                    class: "flex-1",
                    label { class: "block text-sm text-neutral-400 mb-1", "幅" }
                    input {
                        r#type: "number",
                        class: "w-full px-3 py-2 bg-neutral-800 rounded outline-none border border-neutral-700 focus:border-neutral-500",
                        value: "{width}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<f64>() {
                                cfg.write().window.width = v;
                            }
                        },
                    }
                }
                div {
                    class: "flex-1",
                    label { class: "block text-sm text-neutral-400 mb-1", "高さ" }
                    input {
                        r#type: "number",
                        class: "w-full px-3 py-2 bg-neutral-800 rounded outline-none border border-neutral-700 focus:border-neutral-500",
                        value: "{height}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<f64>() {
                                cfg.write().window.height = v;
                            }
                        },
                    }
                }
            }

            // 透明度
            div {
                label {
                    class: "block text-sm text-neutral-400 mb-1",
                    "背景の不透明度: {opacity:.2}"
                }
                input {
                    r#type: "range",
                    class: "range-slider w-full",
                    style: "background: linear-gradient(to right, #a3a3a3 0%, #a3a3a3 {opacity_pct}%, #404040 {opacity_pct}%, #404040 100%);",
                    min: "0.3",
                    max: "1",
                    step: "0.05",
                    value: "{opacity}",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<f64>() {
                            cfg.write().window.opacity = v;
                        }
                    },
                }
            }

            // プロバイダのON/OFF
            div {
                label { class: "block text-sm text-neutral-400 mb-2", "有効にする機能" }
                div {
                    class: "space-y-2",
                    div {
                        class: "flex items-center justify-between",
                        ProviderToggle {
                            label: "クリップボード履歴",
                            checked: p.clipboard,
                            on_toggle: move |v| cfg.write().providers.clipboard = v,
                        }
                        button {
                            class: "px-2 py-1 text-xs text-neutral-400 hover:text-neutral-100 disabled:opacity-40 disabled:hover:text-neutral-400",
                            disabled: clipboard_count == 0,
                            onclick: move |_| {
                                clipboard_history.borrow_mut().clear();
                                message.set("クリップボード履歴を削除しました".to_string());
                            },
                            "履歴を削除 ({clipboard_count}件)"
                        }
                    }
                    ProviderToggle {
                        label: "ウィンドウ切り替え",
                        checked: p.window,
                        on_toggle: move |v| cfg.write().providers.window = v,
                    }
                    ProviderToggle {
                        label: "プロジェクト",
                        checked: p.project,
                        on_toggle: move |v| cfg.write().providers.project = v,
                    }
                    ProviderToggle {
                        label: "ブックマーク",
                        checked: p.bookmark,
                        on_toggle: move |v| cfg.write().providers.bookmark = v,
                    }
                    ProviderToggle {
                        label: "スニペット",
                        checked: p.snippet,
                        on_toggle: move |v| cfg.write().providers.snippet = v,
                    }
                    ProviderToggle {
                        label: "コマンド実行",
                        checked: p.command,
                        on_toggle: move |v| cfg.write().providers.command = v,
                    }
                    ProviderToggle {
                        label: "アプリ起動",
                        checked: p.app,
                        on_toggle: move |v| cfg.write().providers.app = v,
                    }
                    ProviderToggle {
                        label: "Web検索",
                        checked: p.websearch,
                        on_toggle: move |v| cfg.write().providers.websearch = v,
                    }
                    ProviderToggle {
                        label: "AIに聞く",
                        checked: p.llm,
                        on_toggle: move |v| cfg.write().providers.llm = v,
                    }
                }
            }

            // AI (Ollama)
            div {
                label { class: "block text-sm text-neutral-400 mb-2", "AI (Ollama)" }
                div {
                    class: "space-y-2",
                    input {
                        class: "w-full px-3 py-2 bg-neutral-800 rounded outline-none border border-neutral-700 focus:border-neutral-500",
                        placeholder: "ホスト (例: http://localhost:11434)",
                        value: "{llm_host}",
                        oninput: move |e| cfg.write().llm.host = e.value(),
                    }
                    input {
                        class: "w-full px-3 py-2 bg-neutral-800 rounded outline-none border border-neutral-700 focus:border-neutral-500",
                        placeholder: "モデル名 (例: llama3.2)",
                        value: "{llm_model}",
                        oninput: move |e| cfg.write().llm.model = e.value(),
                    }
                }
            }
        }

        // フッター
        div {
            class: "flex items-center justify-between px-6 py-3",
            span { class: "text-xs text-neutral-500", "{current_message}" }
            button {
                class: "px-4 py-2 bg-neutral-700 hover:bg-neutral-600 rounded text-sm",
                onclick: move |_| {
                    let snapshot = cfg.read().clone();
                    match config::save(&snapshot) {
                        Ok(()) => view.set(View::Search),
                        Err(err) => message.set(format!("保存に失敗: {err}")),
                    }
                },
                "保存"
            }
        }
    }
}

/// トグルスイッチ1行分
#[component]
fn ProviderToggle(label: String, checked: bool, on_toggle: EventHandler<bool>) -> Element {
    rsx! {
        label {
            class: "flex items-center gap-3 cursor-pointer select-none",
            span {
                class: "relative inline-flex h-6 w-11 shrink-0",
                input {
                    r#type: "checkbox",
                    class: "peer sr-only",
                    checked,
                    onchange: move |e| on_toggle.call(e.checked()),
                }
                span {
                    class: "absolute inset-0 rounded-full bg-neutral-700 peer-checked:bg-neutral-300 transition-colors",
                }
                span {
                    class: "absolute left-1 top-1 h-4 w-4 rounded-full bg-neutral-100 peer-checked:bg-neutral-900 transition-[transform,background-color] peer-checked:translate-x-5",
                }
            }
            span { class: "text-sm", "{label}" }
        }
    }
}
