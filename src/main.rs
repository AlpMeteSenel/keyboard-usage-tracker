#![cfg_attr(windows, windows_subsystem = "windows")]

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use rusqlite::Connection;

use keyboard_usage_tracker::db::{flush_keys, flush_mouse, init_db};
use keyboard_usage_tracker::events::InputEvent;
use keyboard_usage_tracker::platform;
use keyboard_usage_tracker::platform::key_name;
use keyboard_usage_tracker::dashboard;

fn main() {
    if !platform::ensure_single_instance() {
        platform::open_browser("http://127.0.0.1:9898");
        return;
    }

    let path = platform::db_path();

    {
        let conn = Connection::open(&path).expect("Failed to open database");
        init_db(&conn);
    }

    dashboard::start_dashboard(Arc::new(path.clone()));

    let (tx, rx) = unbounded::<InputEvent>();

    // --- Worker thread: receives events, batch-writes to SQLite ---
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
                Ok(event) => match event {
                    InputEvent::KeyDown {
                        vk_code,
                        is_extended,
                        shift_held,
                        caps_on,
                        ..
                    } => {
                        let name = key_name(vk_code, is_extended);
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
                },
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    for (_, (pressed_at, raw_vk_code, name, shift, caps)) in held_since.drain() {
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

    // Auto-open dashboard in the default browser
    platform::open_browser("http://127.0.0.1:9898");

    // Run platform-specific capture (blocks until shutdown)
    platform::run_capture(tx);

    let _ = worker.join();

    if platform::should_restart() {
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
