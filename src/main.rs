#![cfg_attr(windows, windows_subsystem = "windows")]

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use rusqlite::Connection;

use keyboard_usage_tracker::dashboard;
use keyboard_usage_tracker::db::{
    flush_keys, flush_mouse, flush_touchpad, flush_touchpad_fingers, init_db,
};
use keyboard_usage_tracker::events::InputEvent;
use keyboard_usage_tracker::platform;
use keyboard_usage_tracker::platform::key_name;

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

    // --- WebSocket thread: broadcasts touchpad contacts ---
    thread::spawn(move || {
        if let Ok(server) = TcpListener::bind("127.0.0.1:9899") {
            for stream in server.incoming().flatten() {
                thread::spawn(move || {
                    // Validate token from the query string during the WS handshake
                    let callback = |req: &tungstenite::handshake::server::Request,
                                    resp: tungstenite::handshake::server::Response|
                     -> Result<
                        tungstenite::handshake::server::Response,
                        tungstenite::handshake::server::ErrorResponse,
                    > {
                        let token = req
                            .uri()
                            .query()
                            .unwrap_or("")
                            .split('&')
                            .find(|p| p.starts_with("token="))
                            .and_then(|p| p.strip_prefix("token="))
                            .unwrap_or("");
                        if !dashboard::validate_stats_token(token) {
                            let mut err = tungstenite::handshake::server::ErrorResponse::new(None);
                            *err.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
                            return Err(err);
                        }
                        Ok(resp)
                    };
                    if let Ok(mut websocket) = tungstenite::accept_hdr(stream, callback) {
                        let mut last_json = String::new();
                        loop {
                            let contacts = platform::live_touchpad();
                            let json = serde_json::to_string(&contacts).unwrap_or_default();
                            // Sending continuously ensures the client clears any timeout for actively tracking frames.
                            // We only suppress if NO contacts are pressed AND it's identical (which means suppressing repeated "[]").
                            if !contacts.is_empty() || json != last_json {
                                if websocket
                                    .send(tungstenite::Message::Text(json.clone().into()))
                                    .is_err()
                                {
                                    break;
                                }
                                last_json = json;
                            }
                            thread::sleep(Duration::from_millis(33)); // ~30fps
                        }
                    }
                });
            }
        }
    });

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
        let mut heatmap_buf: HashMap<(i32, i32), u32> = HashMap::new();
        let mut fingers_buf: HashMap<u32, u32> = HashMap::new();
        let mut last_flush = Instant::now();
        let mut last_tp_sample = Instant::now();
        let flush_interval = Duration::from_secs(2);
        let sample_interval = Duration::from_millis(50); // Sub-sample touchpad heavily

        loop {
            // Keep recv_timeout small so we don't miss touchpad sampling
            let wait_time = sample_interval.saturating_sub(last_tp_sample.elapsed());
            let wait_time = if wait_time.is_zero() {
                Duration::from_millis(1)
            } else {
                wait_time
            };

            match rx.recv_timeout(wait_time) {
                Ok(event) => match event {
                    InputEvent::KeyDown {
                        vk_code,
                        is_extended,
                        shift_held,
                        caps_on,
                        ..
                    } => {
                        let name = key_name(vk_code, is_extended);
                        held_since.entry((vk_code, is_extended)).or_insert((
                            Instant::now(),
                            vk_code,
                            name,
                            shift_held,
                            caps_on,
                        ));
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
                    flush_touchpad(&conn, &mut heatmap_buf);
                    flush_touchpad_fingers(&conn, &mut fingers_buf);
                    break;
                }
            }

            if last_tp_sample.elapsed() >= sample_interval {
                let contacts = platform::live_touchpad();
                let count = contacts.len() as u32;
                if count > 0 {
                    *fingers_buf.entry(count).or_default() += 1;
                    for contact in contacts {
                        let grid_x = (contact.x / 20) * 20;
                        let grid_y = (contact.y / 20) * 20;
                        *heatmap_buf.entry((grid_x, grid_y)).or_default() += 1;
                    }
                }
                last_tp_sample = Instant::now();
            }

            if last_flush.elapsed() >= flush_interval
                || key_buf.len() >= 50
                || mouse_buf.len() >= 50
                || heatmap_buf.len() >= 200
            {
                flush_keys(&conn, &mut key_buf);
                flush_mouse(&conn, &mut mouse_buf);
                flush_touchpad(&conn, &mut heatmap_buf);
                flush_touchpad_fingers(&conn, &mut fingers_buf);
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
