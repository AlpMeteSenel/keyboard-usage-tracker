use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Sender;
use evdev::{Device, InputEventKind, Key};

use crate::events::{InputEvent, MouseButton};

fn is_keyboard(device: &Device) -> bool {
    device.supported_keys().map_or(false, |keys| {
        keys.contains(Key::KEY_A) && keys.contains(Key::KEY_Z)
    })
}

fn is_mouse(device: &Device) -> bool {
    let has_buttons = device
        .supported_keys()
        .map_or(false, |keys| keys.contains(Key::BTN_LEFT));
    let has_rel = device
        .supported_relative_axes()
        .map_or(false, |axes| axes.iter().next().is_some());
    has_buttons && has_rel
}

pub fn start_evdev_capture(tx: Sender<InputEvent>) {
    let shift_held = Arc::new(AtomicBool::new(false));
    let caps_on = Arc::new(AtomicBool::new(false));

    let mut found_any = false;

    for (_path, device) in evdev::enumerate() {
        let is_kb = is_keyboard(&device);
        let is_m = is_mouse(&device);

        if !is_kb && !is_m {
            continue;
        }

        found_any = true;
        let tx = tx.clone();
        let shift = Arc::clone(&shift_held);
        let caps = Arc::clone(&caps_on);

        std::thread::spawn(move || {
            let mut device = device;
            let fd = device.as_raw_fd();
            loop {
                // Check if we should stop before blocking on I/O
                if super::SHOULD_STOP.load(Ordering::Relaxed) {
                    break;
                }

                // Poll the fd with a 200ms timeout so we can check SHOULD_STOP periodically
                let mut pollfd = libc::pollfd {
                    fd,
                    events: libc::POLLIN,
                    revents: 0,
                };
                let ret = unsafe { libc::poll(&mut pollfd, 1, 200) };
                if ret <= 0 {
                    continue; // timeout or error — re-check SHOULD_STOP
                }

                match device.fetch_events() {
                    Ok(events) => {
                        for ev in events {
                            if let InputEventKind::Key(key) = ev.kind() {
                                let value = ev.value();

                                // Track modifier state
                                if key == Key::KEY_LEFTSHIFT || key == Key::KEY_RIGHTSHIFT {
                                    shift.store(value != 0, Ordering::Relaxed);
                                }
                                if key == Key::KEY_CAPSLOCK && value == 1 {
                                    let prev = caps.load(Ordering::Relaxed);
                                    caps.store(!prev, Ordering::Relaxed);
                                }

                                // Mouse buttons
                                if key == Key::BTN_LEFT
                                    || key == Key::BTN_RIGHT
                                    || key == Key::BTN_MIDDLE
                                {
                                    if value == 1 {
                                        let button = if key == Key::BTN_LEFT {
                                            MouseButton::Left
                                        } else if key == Key::BTN_RIGHT {
                                            MouseButton::Right
                                        } else {
                                            MouseButton::Middle
                                        };
                                        let _ = tx.try_send(InputEvent::MouseClick {
                                            button,
                                            x: 0,
                                            y: 0,
                                        });
                                    }
                                    continue;
                                }

                                // Keyboard events (ignore repeats, value == 2)
                                if value == 1 {
                                    let _ = tx.try_send(InputEvent::KeyDown {
                                        vk_code: key.0 as u32,
                                        is_extended: false,
                                        shift_held: shift.load(Ordering::Relaxed),
                                        caps_on: caps.load(Ordering::Relaxed),
                                    });
                                } else if value == 0 {
                                    let _ = tx.try_send(InputEvent::KeyUp {
                                        vk_code: key.0 as u32,
                                        is_extended: false,
                                    });
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    if !found_any {
        eprintln!(
            "Warning: No input devices found. \
             Make sure you have read access to /dev/input/event* \
             (add your user to the 'input' group)."
        );
    }
}
