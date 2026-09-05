use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, IsIconic,
    IsWindowVisible, SetForegroundWindow, ShowWindow, GWL_EXSTYLE, GW_OWNER, SW_RESTORE,
    WS_EX_TOOLWINDOW,
};

use crate::provider::{Action, Candidate, Provider};

/// 開いているウィンドウ1件
#[derive(Debug, Clone)]
struct WindowEntry {
    title: String,
    hwnd: isize,
}

pub struct WindowProvider {
    windows: Vec<WindowEntry>,
}

impl WindowProvider {
    pub fn new() -> Self {
        Self {
            windows: enumerate(),
        }
    }
}

impl Provider for WindowProvider {
    fn name(&self) -> &'static str {
        "Window"
    }

    fn reload(&mut self) {
        self.windows = enumerate();
    }

    fn candidates(&self, query: &str) -> Vec<Candidate> {
        let q = query.trim().to_lowercase();

        self.windows
            .iter()
            .filter(|w| q.is_empty() || w.title.to_lowercase().contains(&q))
            .map(|w| Candidate {
                label: format!("{} に切り替え", w.title),
                source: "Window",
                action: Action::FocusWindow(w.hwnd),
            })
            .collect()
    }
}

/// 指定したウィンドウを前面に出す
pub fn focus(hwnd: isize) {
    let hwnd = HWND(hwnd as *mut _);
    unsafe {
        // 最小化されていれば元のサイズに戻す
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

/// タスクバーに並ぶようなウィンドウだけを集める
fn enumerate() -> Vec<WindowEntry> {
    let mut result: Vec<WindowEntry> = Vec::new();
    let ptr = &mut result as *mut Vec<WindowEntry> as isize;

    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(ptr));
    }

    result
}

unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<WindowEntry>);

    // 非表示のウィンドウは除外
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    // オーナーを持つウィンドウ(ダイアログなどの子)は除外
    if GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.is_invalid()) {
        return TRUE;
    }

    // ツールウィンドウ(タスクバーに出ないもの)は除外
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
        return TRUE;
    }

    // DWMでクロークされたウィンドウ(UWPの隠れウィンドウなど)は除外
    let mut cloaked: u32 = 0;
    let ok = DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut u32 as *mut _,
        std::mem::size_of::<u32>() as u32,
    );
    if ok.is_ok() && cloaked != 0 {
        return TRUE;
    }

    // タイトルが無いものは除外
    let len = GetWindowTextLengthW(hwnd);
    if len == 0 {
        return TRUE;
    }

    let mut buf = vec![0u16; (len + 1) as usize];
    let read = GetWindowTextW(hwnd, &mut buf);
    if read == 0 {
        return TRUE;
    }

    let title = String::from_utf16_lossy(&buf[..read as usize]);

    // Poppy 自身は除外
    if title == "Poppy" {
        return TRUE;
    }

    list.push(WindowEntry {
        title,
        hwnd: hwnd.0 as isize,
    });

    TRUE
}
