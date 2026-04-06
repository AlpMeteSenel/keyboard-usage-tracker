mod hooks;
mod keymap;
mod tray;

use std::cell::RefCell;
use std::ffi::c_void;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use crossbeam_channel::Sender;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, MSG,
    WH_KEYBOARD_LL, WH_MOUSE_LL,
};

use crate::events::InputEvent;

static SHOULD_RESTART: AtomicBool = AtomicBool::new(false);
static TRAY_HWND_PTR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

thread_local! {
    static TX: RefCell<Option<Sender<InputEvent>>> = const { RefCell::new(None) };
}

#[inline(always)]
fn emit(event: InputEvent) {
    TX.with(|tx| {
        if let Some(sender) = tx.borrow().as_ref() {
            let _ = sender.try_send(event);
        }
    });
}

use std::sync::{Arc, RwLock};

lazy_static::lazy_static! {
    pub(crate) static ref TOUCHPAD_CONTACTS: Arc<RwLock<(std::time::Instant, Vec<crate::events::TouchpadContact>)>> = Arc::new(RwLock::new((std::time::Instant::now(), Vec::new())));
}

pub fn live_touchpad() -> Vec<crate::events::TouchpadContact> {
    let (last_upd, contacts) = TOUCHPAD_CONTACTS.read().unwrap().clone();
    if last_upd.elapsed() > std::time::Duration::from_millis(250) {
        return Vec::new();
    }
    contacts
}

pub fn ensure_single_instance() -> bool {
    let mutex_name: Vec<u16> = "Global\\KeyboardUsageTracker_SingleInstance\0"
        .encode_utf16()
        .collect();
    let _mutex = unsafe { CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr())) };
    let already_exists =
        unsafe { windows::Win32::Foundation::GetLastError() } == ERROR_ALREADY_EXISTS;
    !already_exists
}

pub fn db_path() -> PathBuf {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
            p.pop();
            p
        });
    let dir = dir.join("keyboard-usage-tracker");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("tracker.db")
}

pub fn key_name(code: u32, extended: bool) -> String {
    keymap::vk_name(code, extended)
}

pub fn hardware_fingerprint() -> String {
    let hostname = std::env::var("COMPUTERNAME").unwrap_or_default();

    let vol_serial: u32 = {
        use windows::Win32::Storage::FileSystem::GetVolumeInformationW;
        let wide_root: Vec<u16> = "C:\\".encode_utf16().chain(std::iter::once(0)).collect();
        let mut serial: u32 = 0;
        unsafe {
            let _ = GetVolumeInformationW(
                windows::core::PCWSTR(wide_root.as_ptr()),
                None,
                Some(&mut serial),
                None,
                None,
                None,
            );
        }
        serial
    };

    format!("{hostname}|{vol_serial:08x}")
}

pub fn open_browser(url: &str) {
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "", url])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn();
}

pub fn signal_restart() {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
    let ptr = TRAY_HWND_PTR.load(Ordering::SeqCst);
    if !ptr.is_null() {
        unsafe {
            let _ = PostMessageW(Some(HWND(ptr)), tray::WM_APP_RESTART, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn signal_stop() {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
    let ptr = TRAY_HWND_PTR.load(Ordering::SeqCst);
    if !ptr.is_null() {
        unsafe {
            let _ = PostMessageW(Some(HWND(ptr)), tray::WM_APP_STOP, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn should_restart() -> bool {
    SHOULD_RESTART.load(Ordering::SeqCst)
}

pub fn run_capture(tx: Sender<InputEvent>) {
    unsafe {
        let _tray_hwnd = tray::setup_tray();

        TX.with(|slot| {
            *slot.borrow_mut() = Some(tx);
        });

        let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hooks::keyboard_hook), None, 0)
            .expect("Failed to install keyboard hook");

        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hooks::mouse_hook), None, 0)
            .expect("Failed to install mouse hook");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(kb_hook);
        let _ = UnhookWindowsHookEx(mouse_hook);

        TX.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}
