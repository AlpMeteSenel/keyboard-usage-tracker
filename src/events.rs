use crossbeam_channel::Sender;

#[derive(Debug, Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyDown {
        vk_code: u32,
        is_extended: bool,
        shift_held: bool,
        caps_on: bool,
    },
    KeyUp {
        vk_code: u32,
        is_extended: bool,
    },
    MouseClick {
        button: MouseButton,
        x: i32,
        y: i32,
    },
}

// ---------------------------------------------------------------------------
// Thread local event emitter
// ---------------------------------------------------------------------------

thread_local! {
    pub static TX: std::cell::RefCell<Option<Sender<InputEvent>>> = const { std::cell::RefCell::new(None) };
}

#[inline(always)]
pub fn emit(event: InputEvent) {
    TX.with(|tx| {
        if let Some(sender) = tx.borrow().as_ref() {
            let _ = sender.try_send(event);
        }
    });
}
