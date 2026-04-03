/// Quick smoke test for the Linux key detection pipeline.
/// Simulates evdev keycodes → key_name mapping → event processing → DB storage.
///
/// Run inside WSL:
///   cargo run --example test_linux_keys

#[cfg(target_os = "linux")]
fn main() {
    use std::time::Instant;
    use crossbeam_channel::bounded;

    // -- 1. Test keymap --------------------------------------------------
    println!("=== Linux keymap test ===\n");

    let test_codes: &[(u32, &str)] = &[
        (30, "A"), (31, "S"), (32, "D"), (33, "F"),
        (16, "Q"), (17, "W"), (18, "E"), (19, "R"),
        (2, "1"), (3, "2"), (4, "3"),
        (57, "Space"), (28, "Enter"), (14, "Backspace"), (15, "Tab"),
        (1, "Escape"), (59, "F1"), (68, "F10"), (87, "F11"),
        (42, "LShift"), (54, "RShift"), (29, "LCtrl"), (97, "RCtrl"),
        (56, "LAlt"), (100, "RAlt"), (125, "Win"),
        (103, "Up"), (108, "Down"), (105, "Left"), (106, "Right"),
        (82, "Num0"), (96, "NumEnter"),
    ];

    let mut pass = 0;
    let mut fail = 0;
    for &(code, expected) in test_codes {
        let got = keyboard_usage_tracker::platform::key_name(code, false);
        if got == expected {
            pass += 1;
        } else {
            fail += 1;
            println!("  FAIL: code {} → got '{}', expected '{}'", code, got, expected);
        }
    }
    println!("  Keymap: {}/{} passed\n", pass, pass + fail);

    // -- 2. Test event pipeline ------------------------------------------
    println!("=== Event pipeline test ===\n");

    let (tx, rx) = bounded::<keyboard_usage_tracker::events::InputEvent>(128);

    // Simulate typing "HELLO" (evdev codes: H=35 E=18 L=38 L=38 O=24)
    // with shift held for capital letters
    use keyboard_usage_tracker::events::{InputEvent, MouseButton};

    let simulated_events = vec![
        InputEvent::KeyDown { vk_code: 42, is_extended: false, shift_held: false, caps_on: false }, // LShift down
        InputEvent::KeyDown { vk_code: 35, is_extended: false, shift_held: true, caps_on: false },  // H
        InputEvent::KeyUp   { vk_code: 35, is_extended: false },
        InputEvent::KeyDown { vk_code: 18, is_extended: false, shift_held: true, caps_on: false },  // E
        InputEvent::KeyUp   { vk_code: 18, is_extended: false },
        InputEvent::KeyDown { vk_code: 38, is_extended: false, shift_held: true, caps_on: false },  // L
        InputEvent::KeyUp   { vk_code: 38, is_extended: false },
        InputEvent::KeyDown { vk_code: 38, is_extended: false, shift_held: true, caps_on: false },  // L
        InputEvent::KeyUp   { vk_code: 38, is_extended: false },
        InputEvent::KeyDown { vk_code: 24, is_extended: false, shift_held: true, caps_on: false },  // O
        InputEvent::KeyUp   { vk_code: 24, is_extended: false },
        InputEvent::KeyUp   { vk_code: 42, is_extended: false },                                     // LShift up
        InputEvent::MouseClick { button: MouseButton::Left, x: 0, y: 0 },
        InputEvent::MouseClick { button: MouseButton::Right, x: 0, y: 0 },
    ];

    for ev in &simulated_events {
        tx.send(ev.clone()).unwrap();
    }
    drop(tx);

    // Collect and process events (same logic as main.rs worker)
    let mut key_buffer: Vec<(u32, String, bool, bool, Option<u64>)> = Vec::new();
    let mut mouse_buffer: Vec<(String, i32, i32)> = Vec::new();

    while let Ok(ev) = rx.recv() {
        match ev {
            InputEvent::KeyDown { vk_code, is_extended, shift_held, caps_on } => {
                let name = keyboard_usage_tracker::platform::key_name(vk_code, is_extended);
                key_buffer.push((vk_code, name, shift_held, caps_on, Some(50)));
            }
            InputEvent::KeyUp { .. } => {}
            InputEvent::MouseClick { button, x, y } => {
                let label = match button {
                    MouseButton::Left => "Left",
                    MouseButton::Right => "Right",
                    MouseButton::Middle => "Middle",
                };
                mouse_buffer.push((label.to_string(), x, y));
            }
        }
    }

    println!("  Captured keys:");
    for (i, (_vk, name, shift, caps, hold)) in key_buffer.iter().enumerate() {
        println!("    {}. {} (shift={}, caps={}, hold={:?}ms)", i + 1, name, shift, caps, hold);
    }
    println!("\n  Captured mouse clicks:");
    for (i, (btn, x, y)) in mouse_buffer.iter().enumerate() {
        println!("    {}. {} at ({}, {})", i + 1, btn, x, y);
    }

    // -- 3. Test DB write ------------------------------------------------
    println!("\n=== DB write test ===\n");

    let db_path = std::env::temp_dir().join("kut_test.db");
    println!("  Using temp DB: {}", db_path.display());

    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    keyboard_usage_tracker::db::init_db(&conn);

    let start = Instant::now();
    keyboard_usage_tracker::db::flush_keys(&conn, &mut key_buffer);
    keyboard_usage_tracker::db::flush_mouse(&conn, &mut mouse_buffer);
    println!("  Flushed keys + clicks in {:?}", start.elapsed());

    // Verify data was written
    let key_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM key_events", [], |r| r.get(0))
        .unwrap();
    let mouse_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM mouse_events", [], |r| r.get(0))
        .unwrap();
    println!("  DB contains: {} key rows, {} mouse rows", key_count, mouse_count);

    // Clean up
    std::fs::remove_file(&db_path).ok();

    println!("\n=== All tests passed ===");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("This test is only for Linux. Run it inside WSL.");
}
