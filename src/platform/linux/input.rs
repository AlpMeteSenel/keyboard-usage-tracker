use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;
use evdev::{AbsoluteAxisType, Device, InputEventKind, Key};

use crate::events::{InputEvent, MouseButton, TouchpadContact};

/// Queries the current cursor position from X11.
/// Returns (0, 0) if X11 is not available (e.g. pure Wayland or headless).
struct CursorTracker {
    display: *mut x11::xlib::Display,
}

impl CursorTracker {
    fn new() -> Option<Self> {
        let display = unsafe { x11::xlib::XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            None
        } else {
            Some(Self { display })
        }
    }

    fn query_position(&self) -> (i32, i32) {
        unsafe {
            let root = x11::xlib::XDefaultRootWindow(self.display);
            let mut root_ret = 0u64;
            let mut child_ret = 0u64;
            let mut root_x = 0i32;
            let mut root_y = 0i32;
            let mut win_x = 0i32;
            let mut win_y = 0i32;
            let mut mask = 0u32;

            x11::xlib::XQueryPointer(
                self.display,
                root,
                &mut root_ret,
                &mut child_ret,
                &mut root_x,
                &mut root_y,
                &mut win_x,
                &mut win_y,
                &mut mask,
            );

            (root_x, root_y)
        }
    }
}

impl Drop for CursorTracker {
    fn drop(&mut self) {
        unsafe {
            x11::xlib::XCloseDisplay(self.display);
        }
    }
}

// Each thread opens its own X11 display connection, so this is safe.
unsafe impl Send for CursorTracker {}

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

fn is_touchpad(device: &Device) -> bool {
    device.supported_absolute_axes().map_or(false, |axes| {
        axes.contains(AbsoluteAxisType::ABS_MT_POSITION_X)
            && axes.contains(AbsoluteAxisType::ABS_MT_POSITION_Y)
            && axes.contains(AbsoluteAxisType::ABS_MT_SLOT)
    })
}

pub fn start_evdev_capture(tx: Sender<InputEvent>) {
    let shift_held = Arc::new(AtomicBool::new(false));
    let caps_on = Arc::new(AtomicBool::new(false));

    let mut found_any = false;

    for (_path, device) in evdev::enumerate() {
        let is_kb = is_keyboard(&device);
        let is_m = is_mouse(&device);
        let is_tp = is_touchpad(&device);

        if !is_kb && !is_m && !is_tp {
            continue;
        }

        found_any = true;

        // Touchpad devices get a dedicated multitouch capture thread
        if is_tp {
            let contacts = Arc::clone(&super::TOUCHPAD_CONTACTS);
            std::thread::spawn(move || {
                run_touchpad_thread(device, contacts);
            });
            continue;
        }

        let tx = tx.clone();
        let shift = Arc::clone(&shift_held);
        let caps = Arc::clone(&caps_on);

        std::thread::spawn(move || {
            let mut device = device;
            let fd = device.as_raw_fd();

            // Open an X11 connection for cursor position queries (one per thread).
            // Falls back to (0, 0) if X11 is unavailable.
            let cursor = if is_m { CursorTracker::new() } else { None };
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
                                        let (x, y) = cursor
                                            .as_ref()
                                            .map(|c| c.query_position())
                                            .unwrap_or((0, 0));
                                        let _ =
                                            tx.try_send(InputEvent::MouseClick { button, x, y });
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

/// Multitouch Type B protocol capture thread.
///
/// Reads ABS_MT_SLOT / ABS_MT_TRACKING_ID / ABS_MT_POSITION_X/Y events
/// from the touchpad device and updates the shared contacts vector on each
/// SYN_REPORT frame boundary.
fn run_touchpad_thread(
    device: Device,
    contacts: Arc<RwLock<(std::time::Instant, Vec<TouchpadContact>)>>,
) {
    let mut device = device;
    let fd = device.as_raw_fd();

    const MAX_SLOTS: usize = 10;
    let mut current_slot: usize = 0;
    // Per-slot state: (tracking_id, x, y). tracking_id == -1 means inactive.
    let mut slots = [(-1i32, 0i32, 0i32); MAX_SLOTS];
    let mut dirty = false;

    loop {
        if super::SHOULD_STOP.load(Ordering::Relaxed) {
            break;
        }

        // Poll with timeout so we can check SHOULD_STOP periodically
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pollfd, 1, 200) };
        if ret <= 0 {
            continue; // timeout or error
        }

        match device.fetch_events() {
            Ok(events) => {
                for ev in events {
                    match ev.kind() {
                        InputEventKind::AbsAxis(axis) => {
                            if axis == AbsoluteAxisType::ABS_MT_SLOT {
                                let s = ev.value() as usize;
                                if s < MAX_SLOTS {
                                    current_slot = s;
                                }
                            } else if axis == AbsoluteAxisType::ABS_MT_TRACKING_ID {
                                if current_slot < MAX_SLOTS {
                                    slots[current_slot].0 = ev.value();
                                    // When a finger lifts (tracking_id == -1),
                                    // zero out position so stale coords aren't reused.
                                    if ev.value() == -1 {
                                        slots[current_slot].1 = 0;
                                        slots[current_slot].2 = 0;
                                    }
                                    dirty = true;
                                }
                            } else if axis == AbsoluteAxisType::ABS_MT_POSITION_X {
                                if current_slot < MAX_SLOTS {
                                    slots[current_slot].1 = ev.value();
                                    dirty = true;
                                }
                            } else if axis == AbsoluteAxisType::ABS_MT_POSITION_Y {
                                if current_slot < MAX_SLOTS {
                                    slots[current_slot].2 = ev.value();
                                    dirty = true;
                                }
                            }
                        }
                        InputEventKind::Synchronization(_) => {
                            if dirty {
                                let active: Vec<TouchpadContact> = slots
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, (tid, _, _))| *tid >= 0)
                                    .map(|(i, (_, x, y))| TouchpadContact {
                                        id: i as u32,
                                        x: *x,
                                        y: *y,
                                    })
                                    .collect();
                                if let Ok(mut w) = contacts.write() {
                                    *w = (std::time::Instant::now(), active);
                                }
                                dirty = false;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Clear contacts on thread exit
    if let Ok(mut w) = contacts.write() {
        w.1.clear();
    }
}
