#![windows_subsystem = "windows"]

use std::os::windows::process::CommandExt;

mod dashboard;
mod db;
mod events;
mod hooks;
mod keymap;
mod tray;

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use rusqlite::Connection;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    MSG, WH_KEYBOARD_LL, WH_MOUSE_LL,
};

use crate::db::{flush_keys, flush_mouse};
use crate::events::{InputEvent, TX};
use crate::keymap::vk_name;
use crate::tray::SHOULD_RESTART;

fn main() {
    // --- Single instance guard via named mutex ---
    let mutex_name: Vec<u16> = "Global\\KeyboardUsageTracker_SingleInstance\0"
        .encode_utf16()
        .collect();
    let _mutex = unsafe { CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr())) };
    if unsafe { windows::Win32::Foundation::GetLastError() } == ERROR_ALREADY_EXISTS {
        // Another instance is already running, open the dashboard and exit.
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", "http://127.0.0.1:9898"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
        return;
    }

    let path = db::db_path();

    {
        let conn = Connection::open(&path).expect("Failed to open database");
        db::init_db(&conn);
    }

    dashboard::start_dashboard(Arc::new(path.clone()));

    let _tray_hwnd = unsafe { tray::setup_tray() };

    let (tx, rx) = unbounded::<InputEvent>();

    // --- Worker thread: receives events, batchwrites to SQLite ---
    let worker_db_path = path.clone();
    let worker = thread::spawn(move || {
        let conn = Connection::open(&worker_db_path).expect("Failed to open DB in worker");

        let mut key_buf: Vec<(u32, String, bool, bool, Option<u64>)> = Vec::with_capacity(64);
        let mut mouse_buf: Vec<(String, i32, i32)> = Vec::with_capacity(64);
        let mut held_since: HashMap<(u32, bool), (Instant, u32, String, bool, bool)> =
            HashMap::new();
        let mut last_flush = Instant::now();
        let flush_interval = Duration::from_secs(2);

        loop {
            match rx.recv_timeout(flush_interval) {
                Ok(event) => {
                    match event {
                        InputEvent::KeyDown {
                            vk_code,
                            is_extended,
                            shift_held,
                            caps_on,
                            ..
                        } => {
                            let name = vk_name(vk_code, is_extended);
                            held_since
                                .entry((vk_code, is_extended))
                                .or_insert((Instant::now(), vk_code, name, shift_held, caps_on));
                        }
                        InputEvent::KeyUp {
                            vk_code,
                            is_extended,
                        } => {
                            if let Some((pressed_at, raw_vk_code, name, shift, caps)) =
                                held_since.remove(&(vk_code, is_extended))
                            {
                                let hold_ms = pressed_at.elapsed().as_millis() as u64;
                                key_buf.push((raw_vk_code, name, shift, caps, Some(hold_ms)));
                            }
                        }
                        InputEvent::MouseClick { button, x, y } => {
                            let btn = format!("{:?}", button);
                            mouse_buf.push((btn, x, y));
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    for ((_vk_code, _is_extended), (pressed_at, raw_vk_code, name, shift, caps)) in
                        held_since.drain()
                    {
                        let hold_ms = pressed_at.elapsed().as_millis() as u64;
                        key_buf.push((raw_vk_code, name, shift, caps, Some(hold_ms)));
                    }
                    flush_keys(&conn, &mut key_buf);
                    flush_mouse(&conn, &mut mouse_buf);
                    break;
                }
            }

            if last_flush.elapsed() >= flush_interval
                || key_buf.len() >= 50
                || mouse_buf.len() >= 50
            {
                flush_keys(&conn, &mut key_buf);
                flush_mouse(&conn, &mut mouse_buf);
                last_flush = Instant::now();
            }
        }
    });

    // --- Install hooks on the main thread ---
    unsafe {
        TX.with(|slot| {
            *slot.borrow_mut() = Some(tx);
        });

        let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hooks::keyboard_hook), None, 0)
            .expect("Failed to install keyboard hook");

        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(hooks::mouse_hook), None, 0)
            .expect("Failed to install mouse hook");

        // Auto-open dashboard in the default browser
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", "http://127.0.0.1:9898"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();

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

    let _ = worker.join();

    if SHOULD_RESTART.load(Ordering::SeqCst) {
        if let Ok(exe) = std::env::current_exe() {
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if TcpListener::bind("127.0.0.1:9898").is_ok() {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
            let _ = std::process::Command::new(exe).spawn();
        }
    }
}
