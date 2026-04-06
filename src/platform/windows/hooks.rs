use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_CAPITAL, VK_LSHIFT, VK_RSHIFT, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::emit;
use crate::events::{InputEvent, MouseButton};

pub unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let is_extended = (info.flags.0 & 0x01) != 0;
            let shift_held = GetKeyState(VK_SHIFT.0 as i32) < 0
                || GetKeyState(VK_LSHIFT.0 as i32) < 0
                || GetKeyState(VK_RSHIFT.0 as i32) < 0;
            let caps_on = (GetKeyState(VK_CAPITAL.0 as i32) & 1) != 0;
            emit(InputEvent::KeyDown {
                vk_code: info.vkCode,
                is_extended,
                shift_held,
                caps_on,
            });
        } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
            let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            let is_extended = (info.flags.0 & 0x01) != 0;
            emit(InputEvent::KeyUp {
                vk_code: info.vkCode,
                is_extended,
            });
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

pub unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        let msg = wparam.0 as u32;
        match msg {
            WM_LBUTTONDOWN => emit(InputEvent::MouseClick {
                button: MouseButton::Left,
                x: info.pt.x,
                y: info.pt.y,
            }),
            WM_RBUTTONDOWN => emit(InputEvent::MouseClick {
                button: MouseButton::Right,
                x: info.pt.x,
                y: info.pt.y,
            }),
            WM_MBUTTONDOWN => emit(InputEvent::MouseClick {
                button: MouseButton::Middle,
                x: info.pt.x,
                y: info.pt.y,
            }),
            _ => {}
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
