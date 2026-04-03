use std::mem;
use std::ptr;
use std::sync::atomic::Ordering;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GetCursorPos, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, LoadIconW, LoadImageW, MF_STRING,
    PostQuitMessage, RegisterClassW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TrackPopupMenu, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_DESTROY, WM_RBUTTONUP,
    WNDCLASSW,
};

pub(super) const WM_TRAYICON: u32 = 0x8000 + 1;
pub(super) const WM_APP_RESTART: u32 = 0x8000 + 2;
pub(super) const WM_APP_STOP: u32 = 0x8000 + 3;
const IDM_DASHBOARD: usize = 1001;
const IDM_RESTART: usize = 1002;
const IDM_EXIT: usize = 1003;

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_tip(s: &str) -> [u16; 128] {
    let mut buf = [0u16; 128];
    for (i, c) in s.encode_utf16().take(127).enumerate() {
        buf[i] = c;
    }
    buf
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let mouse_msg = (lparam.0 & 0xFFFF) as u32;
            if mouse_msg == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as usize;
            match id {
                IDM_DASHBOARD => {
                    super::open_browser("http://127.0.0.1:9898");
                }
                IDM_RESTART => {
                    super::SHOULD_RESTART.store(true, Ordering::SeqCst);
                    let _ = DestroyWindow(hwnd);
                }
                IDM_EXIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_APP_RESTART => {
            super::SHOULD_RESTART.store(true, Ordering::SeqCst);
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_APP_STOP => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    if let Ok(menu) = CreatePopupMenu() {
        let open_label = wide_null("Open Dashboard");
        let restart_label = wide_null("Restart Tracker");
        let exit_label = wide_null("Exit");
        let _ = AppendMenuW(menu, MF_STRING, IDM_DASHBOARD, PCWSTR(open_label.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_RESTART, PCWSTR(restart_label.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT, PCWSTR(exit_label.as_ptr()));

        let mut pt = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_BOTTOMALIGN | TPM_LEFTALIGN, pt.x, pt.y, Some(0), hwnd, None);
        let _ = DestroyMenu(menu);
    }
}

pub(super) unsafe fn setup_tray() -> Option<HWND> {
    let class_name = wide_null("KBTrackerTray");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(tray_wndproc),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..mem::zeroed()
    };
    let atom = RegisterClassW(&wc);
    if atom == 0 {
        return None;
    }

    let hwnd = match CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        PCWSTR(class_name.as_ptr()),
        PCWSTR(ptr::null()),
        WINDOW_STYLE::default(),
        0,
        0,
        0,
        0,
        None,
        None,
        None,
        None,
    ) {
        Ok(h) => h,
        Err(_) => return None,
    };

    let hinstance = GetModuleHandleW(None).ok();
    let icon = hinstance
        .and_then(|h| {
            LoadImageW(Some(h.into()), PCWSTR(1 as *const u16), IMAGE_ICON, 0, 0, LR_DEFAULTSIZE)
                .ok()
        })
        .and_then(|h| {
            if h.is_invalid() { None } else { Some(h) }
        })
        .map(|h| windows::Win32::UI::WindowsAndMessaging::HICON(h.0))
        .or_else(|| LoadIconW(None, IDI_APPLICATION).ok());
    let icon = match icon {
        Some(i) => i,
        None => return None,
    };

    let mut nid: NOTIFYICONDATAW = mem::zeroed();
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = icon;
    nid.szTip = wide_tip("Keyboard Usage Tracker");

    let _ = Shell_NotifyIconW(NIM_ADD, &nid);

    super::TRAY_HWND_PTR.store(hwnd.0, Ordering::SeqCst);

    Some(hwnd)
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = mem::zeroed();
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}
