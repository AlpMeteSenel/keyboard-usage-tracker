use serde::Serialize;

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

#[derive(Debug, Clone, Serialize)]
pub struct TouchpadContact {
    pub id: u32,
    pub x: i32,
    pub y: i32,
}
