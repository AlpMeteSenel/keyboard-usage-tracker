use std::collections::HashMap;
use std::mem;
use std::ptr;
use std::sync::atomic::Ordering;

use windows::core::PCWSTR;
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidP_GetButtonCaps, HidP_GetCaps, HidP_GetData, HidP_GetValueCaps, HidP_Input,
    HidP_MaxDataListLength, HIDP_BUTTON_CAPS, HIDP_CAPS, HIDP_DATA, HIDP_VALUE_CAPS,
    PHIDP_PREPARSED_DATA,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, RegisterRawInputDevices, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RIDEV_INPUTSINK, RIDI_PREPARSEDDATA, RID_INPUT, RIM_TYPEHID,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GetCursorPos, LoadIconW, LoadImageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    TrackPopupMenu, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, MF_STRING, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, WINDOW_EX_STYLE, WINDOW_STYLE, WM_COMMAND, WM_DESTROY, WM_INPUT, WM_RBUTTONUP,
    WNDCLASSW,
};

pub(super) const WM_TRAYICON: u32 = 0x8000 + 1;
pub(super) const WM_APP_RESTART: u32 = 0x8000 + 2;
pub(super) const WM_APP_STOP: u32 = 0x8000 + 3;
const IDM_DASHBOARD: usize = 1001;
const IDM_RESTART: usize = 1002;
const IDM_EXIT: usize = 1003;

#[derive(Default)]
struct ContactInfo {
    id: u32,
    x: i32,
    y: i32,
    tip_switch: Option<u32>,
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_tip(s: &str) -> [u16; 128] {
    let mut buf = [0u16; 128];
    for (i, c) in s.encode_utf16().take(127).enumerate() {
        buf[i] = c;
    }
    buf
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_INPUT => {
            let mut size = 0;
            let _ = GetRawInputData(
                windows::Win32::UI::Input::HRAWINPUT(lparam.0 as *mut _),
                RID_INPUT,
                None,
                &mut size,
                mem::size_of::<RAWINPUTHEADER>() as u32,
            );

            if size > 0 {
                let mut buf = vec![0u8; size as usize];
                if GetRawInputData(
                    windows::Win32::UI::Input::HRAWINPUT(lparam.0 as *mut _),
                    RID_INPUT,
                    Some(buf.as_mut_ptr() as *mut _),
                    &mut size,
                    mem::size_of::<RAWINPUTHEADER>() as u32,
                ) != u32::MAX
                {
                    let raw = &mut *(buf.as_mut_ptr() as *mut RAWINPUT);
                    if raw.header.dwType == RIM_TYPEHID.0 {
                        let mut pcb_size = 0;
                        let _ = GetRawInputDeviceInfoW(
                            Some(raw.header.hDevice),
                            RIDI_PREPARSEDDATA,
                            None,
                            &mut pcb_size,
                        );

                        if pcb_size > 0 {
                            let mut preparsed_buf = vec![0u8; pcb_size as usize];
                            if GetRawInputDeviceInfoW(
                                Some(raw.header.hDevice),
                                RIDI_PREPARSEDDATA,
                                Some(preparsed_buf.as_mut_ptr() as *mut _),
                                &mut pcb_size,
                            ) != u32::MAX
                            {
                                let preparsed_data =
                                    PHIDP_PREPARSED_DATA(preparsed_buf.as_mut_ptr() as isize);
                                let mut caps = core::mem::zeroed::<HIDP_CAPS>();
                                if HidP_GetCaps(preparsed_data, &mut caps).is_ok() {
                                    let mut value_caps_len = caps.NumberInputValueCaps;
                                    let mut value_caps = vec![
                                        core::mem::zeroed::<HIDP_VALUE_CAPS>();
                                        value_caps_len as usize
                                    ];
                                    let _ = HidP_GetValueCaps(
                                        HidP_Input,
                                        value_caps.as_mut_ptr(),
                                        &mut value_caps_len,
                                        preparsed_data,
                                    );

                                    let mut button_caps_len = caps.NumberInputButtonCaps;
                                    let mut button_caps = vec![
                                        core::mem::zeroed::<HIDP_BUTTON_CAPS>(
                                        );
                                        button_caps_len as usize
                                    ];
                                    let _ = HidP_GetButtonCaps(
                                        HidP_Input,
                                        button_caps.as_mut_ptr(),
                                        &mut button_caps_len,
                                        preparsed_data,
                                    );

                                    let data_len =
                                        HidP_MaxDataListLength(HidP_Input, preparsed_data);
                                    if data_len > 0 {
                                        let mut data_list = vec![
                                            core::mem::zeroed::<HIDP_DATA>();
                                            data_len as usize
                                        ];
                                        let mut actual_len = data_len;
                                        let raw_len = (raw.data.hid.dwCount
                                            * raw.data.hid.dwSizeHid)
                                            as usize;
                                        let raw_slice = std::slice::from_raw_parts_mut(
                                            raw.data.hid.bRawData.as_mut_ptr(),
                                            raw_len,
                                        );

                                        if HidP_GetData(
                                            HidP_Input,
                                            data_list.as_mut_ptr(),
                                            &mut actual_len,
                                            preparsed_data,
                                            raw_slice,
                                        )
                                        .is_ok()
                                        {
                                            data_list.truncate(actual_len as usize);
                                            let mut contacts_map: HashMap<u16, ContactInfo> =
                                                HashMap::new();

                                            for d in data_list {
                                                let mut usage_page = 0;
                                                let mut usage = 0;
                                                let mut link_col = 0;
                                                let mut found = false;
                                                let mut is_button = false;

                                                if let Some(cap) = value_caps.iter().find(|c| {
                                                    if c.IsRange.into() {
                                                        d.DataIndex
                                                            >= c.Anonymous.Range.DataIndexMin
                                                            && d.DataIndex
                                                                <= c.Anonymous.Range.DataIndexMax
                                                    } else {
                                                        d.DataIndex
                                                            == c.Anonymous.NotRange.DataIndex
                                                    }
                                                }) {
                                                    usage_page = cap.UsagePage;
                                                    usage = if cap.IsRange.into() {
                                                        cap.Anonymous.Range.UsageMin
                                                            + (d.DataIndex
                                                                - cap.Anonymous.Range.DataIndexMin)
                                                    } else {
                                                        cap.Anonymous.NotRange.Usage
                                                    };
                                                    link_col = cap.LinkCollection;
                                                    found = true;
                                                } else if let Some(cap) =
                                                    button_caps.iter().find(|c| {
                                                        if c.IsRange.into() {
                                                            d.DataIndex
                                                                >= c.Anonymous.Range.DataIndexMin
                                                                && d.DataIndex
                                                                    <= c.Anonymous
                                                                        .Range
                                                                        .DataIndexMax
                                                        } else {
                                                            d.DataIndex
                                                                == c.Anonymous.NotRange.DataIndex
                                                        }
                                                    })
                                                {
                                                    usage_page = cap.UsagePage;
                                                    usage = if cap.IsRange.into() {
                                                        cap.Anonymous.Range.UsageMin
                                                            + (d.DataIndex
                                                                - cap.Anonymous.Range.DataIndexMin)
                                                    } else {
                                                        cap.Anonymous.NotRange.Usage
                                                    };
                                                    link_col = cap.LinkCollection;
                                                    found = true;
                                                    is_button = true;
                                                }

                                                if found {
                                                    let info =
                                                        contacts_map.entry(link_col).or_default();
                                                    let val = if is_button {
                                                        if d.Anonymous.On.into() {
                                                            1
                                                        } else {
                                                            0
                                                        }
                                                    } else {
                                                        d.Anonymous.RawValue
                                                    };

                                                    if usage_page == 0x01 {
                                                        if usage == 0x30 {
                                                            info.x = val as i32;
                                                        }
                                                        if usage == 0x31 {
                                                            info.y = val as i32;
                                                        }
                                                    } else if usage_page == 0x0D {
                                                        if usage == 0x51 {
                                                            info.id = val;
                                                        }
                                                        if usage == 0x42 {
                                                            info.tip_switch = Some(val);
                                                        }
                                                    }
                                                }
                                            }

                                            let mut contacts: Vec<crate::events::TouchpadContact> =
                                                Vec::new();
                                            for (_, info) in contacts_map {
                                                // Strict rule: if Tip Switch exists but is 0, drop it.
                                                // If missing, assume it's pressed as long as X,Y > 0.
                                                let is_pressed = match info.tip_switch {
                                                    Some(t) => t > 0,
                                                    None => true,
                                                };

                                                if is_pressed && info.x > 0 && info.y > 0 {
                                                    contacts.push(crate::events::TouchpadContact {
                                                        id: info.id,
                                                        x: info.x,
                                                        y: info.y,
                                                    });
                                                }
                                            }

                                            if let Ok(mut lock) = super::TOUCHPAD_CONTACTS.write() {
                                                *lock = (std::time::Instant::now(), contacts);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_TRAYICON => {
            let mouse_msg = (lparam.0 & 0xFFFF) as u32;
            if mouse_msg == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as usize;
            match id {
                IDM_DASHBOARD => {
                    super::open_browser("http://127.0.0.1:9898");
                }
                IDM_RESTART => {
                    super::SHOULD_RESTART.store(true, Ordering::SeqCst);
                    let _ = DestroyWindow(hwnd);
                }
                IDM_EXIT => {
                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_APP_RESTART => {
            super::SHOULD_RESTART.store(true, Ordering::SeqCst);
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_APP_STOP => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    if let Ok(menu) = CreatePopupMenu() {
        let open_label = wide_null("Open Dashboard");
        let restart_label = wide_null("Restart Tracker");
        let exit_label = wide_null("Exit");
        let _ = AppendMenuW(menu, MF_STRING, IDM_DASHBOARD, PCWSTR(open_label.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_RESTART, PCWSTR(restart_label.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT, PCWSTR(exit_label.as_ptr()));

        let mut pt = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            menu,
            TPM_BOTTOMALIGN | TPM_LEFTALIGN,
            pt.x,
            pt.y,
            Some(0),
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
    }
}

pub(super) unsafe fn setup_tray() -> Option<HWND> {
    let class_name = wide_null("KBTrackerTray");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(tray_wndproc),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..mem::zeroed()
    };
    let atom = RegisterClassW(&wc);
    if atom == 0 {
        return None;
    }

    let hwnd = match CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        PCWSTR(class_name.as_ptr()),
        PCWSTR(ptr::null()),
        WINDOW_STYLE::default(),
        0,
        0,
        0,
        0,
        None,
        None,
        None,
        None,
    ) {
        Ok(h) => h,
        Err(_) => return None,
    };

    let rid = [RAWINPUTDEVICE {
        usUsagePage: 0x0D,
        usUsage: 0x05,
        dwFlags: RIDEV_INPUTSINK,
        hwndTarget: hwnd,
    }];
    let _ = RegisterRawInputDevices(&rid, mem::size_of::<RAWINPUTDEVICE>() as u32);

    let hinstance = GetModuleHandleW(None).ok();
    let icon = hinstance
        .and_then(|h| {
            LoadImageW(
                Some(h.into()),
                PCWSTR(1 as *const u16),
                IMAGE_ICON,
                0,
                0,
                LR_DEFAULTSIZE,
            )
            .ok()
        })
        .and_then(|h| if h.is_invalid() { None } else { Some(h) })
        .map(|h| windows::Win32::UI::WindowsAndMessaging::HICON(h.0))
        .or_else(|| LoadIconW(None, IDI_APPLICATION).ok());
    let icon = match icon {
        Some(i) => i,
        None => return None,
    };

    let mut nid: NOTIFYICONDATAW = mem::zeroed();
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = icon;
    nid.szTip = wide_tip("Keyboard Usage Tracker");

    let _ = Shell_NotifyIconW(NIM_ADD, &nid);

    super::TRAY_HWND_PTR.store(hwnd.0, Ordering::SeqCst);

    Some(hwnd)
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = mem::zeroed();
    nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}
