mod input;
mod keymap;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use crate::events::InputEvent;

static SHOULD_RESTART: AtomicBool = AtomicBool::new(false);
static SHOULD_STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_sig: libc::c_int) {
    SHOULD_STOP.store(true, Ordering::SeqCst);
}

pub fn ensure_single_instance() -> bool {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    let lock_path = lock_file_path();
    let file = match File::create(&lock_path) {
        Ok(f) => f,
        Err(_) => return true,
    };

    let fd = file.as_raw_fd();
    let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        return false;
    }

    // Leak the file to keep the lock alive for process lifetime
    std::mem::forget(file);
    true
}

fn lock_file_path() -> PathBuf {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    runtime_dir.join("keyboard-usage-tracker.lock")
}

pub fn db_path() -> PathBuf {
    let dir = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            home.join(".local").join("share")
        });
    let dir = dir.join("keyboard-usage-tracker");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("tracker.db")
}

pub fn key_name(code: u32, extended: bool) -> String {
    keymap::key_name(code, extended)
}

pub fn hardware_fingerprint() -> String {
    let hostname = {
        let mut buf = [0u8; 256];
        let ret = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
        if ret == 0 {
            let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            String::from_utf8_lossy(&buf[..len]).into_owned()
        } else {
            String::new()
        }
    };

    let machine_id = std::fs::read_to_string("/etc/machine-id")
        .unwrap_or_default()
        .trim()
        .to_string();

    format!("{hostname}|{machine_id}")
}

pub fn open_browser(url: &str) {
    let _ = std::process::Command::new("xdg-open")
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn signal_restart() {
    SHOULD_RESTART.store(true, Ordering::SeqCst);
    SHOULD_STOP.store(true, Ordering::SeqCst);
}

pub fn signal_stop() {
    SHOULD_STOP.store(true, Ordering::SeqCst);
}

pub fn should_restart() -> bool {
    SHOULD_RESTART.load(Ordering::SeqCst)
}

pub fn run_capture(tx: Sender<InputEvent>) {
    // Install signal handlers for graceful shutdown
    unsafe {
        libc::signal(libc::SIGTERM, handle_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, handle_signal as *const () as libc::sighandler_t);
    }

    input::start_evdev_capture(tx);

    // Block until signaled to stop
    while !SHOULD_STOP.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
