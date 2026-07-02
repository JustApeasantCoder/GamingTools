#[cfg(windows)]
mod windows_input {
    use std::{mem::size_of, thread, time::Duration};
    use windows_sys::Win32::{
        Foundation::POINT,
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT,
                KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
                MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
                MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, VK_ESCAPE,
                VK_LBUTTON, VK_LCONTROL, VK_MBUTTON, VK_MENU, VK_RBUTTON, VK_RCONTROL, VK_RETURN,
                VK_SHIFT, VK_SPACE, VK_TAB, VK_XBUTTON1, VK_XBUTTON2,
            },
            WindowsAndMessaging::{GetCursorPos, SetCursorPos},
        },
    };

    const XBUTTON1_MOUSE_DATA: u32 = 0x0001;
    const XBUTTON2_MOUSE_DATA: u32 = 0x0002;

    #[derive(Clone, Copy, Debug)]
    pub struct ClickTiming {
        pub cursor_settle_ms: u64,
        pub click_hold_ms: u64,
        pub click_release_settle_ms: u64,
    }

    pub fn is_key_down(key: &str) -> bool {
        if let Some(vk) = virtual_key_code(key) {
            unsafe {
                let state =
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk as i32);
                return state & 0x8000u16 as i16 != 0;
            }
        }
        false
    }

    pub fn key_down(key: &str) -> Result<(), String> {
        send_down(key)
    }

    pub fn key_up(key: &str) -> Result<(), String> {
        send_up(key)
    }

    pub fn supports_key(key: &str) -> bool {
        virtual_key_code(key).is_some()
    }

    pub fn supported_key_names() -> Vec<String> {
        let mut keys = ('A'..='Z')
            .chain('0'..='9')
            .map(|key| key.to_string())
            .collect::<Vec<_>>();
        keys.extend((1..=12).map(|number| format!("F{number}")));
        keys.extend(
            [
                "SPACE",
                "ENTER",
                "ESC",
                "TAB",
                "SHIFT",
                "CTRL",
                "RIGHT CTRL",
                "LEFT CTRL",
                "ALT",
                "LEFT CLICK",
                "RIGHT CLICK",
                "MIDDLE CLICK",
                "MOUSE 4",
                "MOUSE 5",
            ]
            .into_iter()
            .map(String::from),
        );
        keys
    }

    pub fn left_click_at(x: i32, y: i32, timing: ClickTiming) -> Result<(), String> {
        move_cursor_to(x, y)?;
        thread::sleep(Duration::from_millis(timing.cursor_settle_ms));

        send_down("LEFT CLICK")?;
        thread::sleep(Duration::from_millis(timing.click_hold_ms));
        let result = send_up("LEFT CLICK");
        if result.is_err() {
            let _ = send_up("LEFT CLICK");
        }
        thread::sleep(Duration::from_millis(timing.click_release_settle_ms));
        result
    }

    pub fn move_cursor_to(x: i32, y: i32) -> Result<(), String> {
        unsafe {
            if SetCursorPos(x, y) == 0 {
                return Err("Unable to move cursor".into());
            }
        }
        Ok(())
    }

    pub fn cursor_position() -> Result<(i32, i32), String> {
        let mut point = POINT { x: 0, y: 0 };
        let result = unsafe { GetCursorPos(&mut point) };
        if result == 0 {
            Err("Unable to read cursor position".into())
        } else {
            Ok((point.x, point.y))
        }
    }

    fn send_down(key: &str) -> Result<(), String> {
        match input_code(key).ok_or_else(|| format!("Unsupported key: {key}"))? {
            InputCode::Keyboard { vk, flags } => send_keyboard(vk, flags, false),
            InputCode::Mouse {
                down, mouse_data, ..
            } => send_mouse(down, mouse_data),
        }
    }

    fn send_up(key: &str) -> Result<(), String> {
        match input_code(key).ok_or_else(|| format!("Unsupported key: {key}"))? {
            InputCode::Keyboard { vk, flags } => send_keyboard(vk, flags, true),
            InputCode::Mouse { up, mouse_data, .. } => send_mouse(up, mouse_data),
        }
    }

    fn send_keyboard(vk: u16, flags: u32, key_up: bool) -> Result<(), String> {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags | if key_up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let sent = unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
        if sent == 1 {
            Ok(())
        } else {
            Err("SendInput failed".into())
        }
    }

    fn send_mouse(flags: u32, mouse_data: u32) -> Result<(), String> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: mouse_data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let sent = unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
        if sent == 1 {
            Ok(())
        } else {
            Err("SendInput failed".into())
        }
    }

    enum InputCode {
        Keyboard {
            vk: u16,
            flags: u32,
        },
        Mouse {
            vk: u16,
            down: u32,
            up: u32,
            mouse_data: u32,
        },
    }

    fn virtual_key_code(key: &str) -> Option<u16> {
        match input_code(key)? {
            InputCode::Keyboard { vk, .. } => Some(vk),
            InputCode::Mouse { vk, .. } => Some(vk),
        }
    }

    fn input_code(key: &str) -> Option<InputCode> {
        let normalized = key.trim().to_uppercase();
        if normalized.len() == 1 {
            let byte = normalized.as_bytes()[0];
            if byte.is_ascii_alphanumeric() {
                return Some(keyboard(byte as u16));
            }
        }

        match normalized.as_str() {
            "F1" => Some(keyboard(0x70)),
            "F2" => Some(keyboard(0x71)),
            "F3" => Some(keyboard(0x72)),
            "F4" => Some(keyboard(0x73)),
            "F5" => Some(keyboard(0x74)),
            "F6" => Some(keyboard(0x75)),
            "F7" => Some(keyboard(0x76)),
            "F8" => Some(keyboard(0x77)),
            "F9" => Some(keyboard(0x78)),
            "F10" => Some(keyboard(0x79)),
            "F11" => Some(keyboard(0x7A)),
            "F12" => Some(keyboard(0x7B)),
            "SPACE" | "SPACEBAR" => Some(keyboard(VK_SPACE)),
            "ENTER" => Some(keyboard(VK_RETURN)),
            "ESC" | "ESCAPE" => Some(keyboard(VK_ESCAPE)),
            "TAB" => Some(keyboard(VK_TAB)),
            "SHIFT" => Some(keyboard(VK_SHIFT)),
            "CTRL" | "CONTROL" | "RIGHT CTRL" | "RIGHT CONTROL" | "RCTRL" | "RCONTROL" => {
                Some(right_control())
            }
            "LEFT CTRL" | "LEFT CONTROL" | "LCTRL" | "LCONTROL" => Some(keyboard(VK_LCONTROL)),
            "ALT" => Some(keyboard(VK_MENU)),
            "LEFT CLICK" => Some(InputCode::Mouse {
                vk: VK_LBUTTON,
                down: MOUSEEVENTF_LEFTDOWN,
                up: MOUSEEVENTF_LEFTUP,
                mouse_data: 0,
            }),
            "RIGHT CLICK" => Some(InputCode::Mouse {
                vk: VK_RBUTTON,
                down: MOUSEEVENTF_RIGHTDOWN,
                up: MOUSEEVENTF_RIGHTUP,
                mouse_data: 0,
            }),
            "MIDDLE CLICK" => Some(InputCode::Mouse {
                vk: VK_MBUTTON,
                down: MOUSEEVENTF_MIDDLEDOWN,
                up: MOUSEEVENTF_MIDDLEUP,
                mouse_data: 0,
            }),
            "MOUSE 4" | "XBUTTON1" => Some(InputCode::Mouse {
                vk: VK_XBUTTON1,
                down: MOUSEEVENTF_XDOWN,
                up: MOUSEEVENTF_XUP,
                mouse_data: XBUTTON1_MOUSE_DATA,
            }),
            "MOUSE 5" | "XBUTTON2" => Some(InputCode::Mouse {
                vk: VK_XBUTTON2,
                down: MOUSEEVENTF_XDOWN,
                up: MOUSEEVENTF_XUP,
                mouse_data: XBUTTON2_MOUSE_DATA,
            }),
            _ => None,
        }
    }

    fn keyboard(vk: u16) -> InputCode {
        InputCode::Keyboard { vk, flags: 0 }
    }

    fn right_control() -> InputCode {
        InputCode::Keyboard {
            vk: VK_RCONTROL,
            flags: KEYEVENTF_EXTENDEDKEY,
        }
    }
}

#[cfg(not(windows))]
mod windows_input {
    #[derive(Clone, Copy, Debug)]
    pub struct ClickTiming {
        pub cursor_settle_ms: u64,
        pub click_hold_ms: u64,
        pub click_release_settle_ms: u64,
    }

    pub fn is_key_down(_key: &str) -> bool {
        false
    }

    pub fn key_down(_key: &str) -> Result<(), String> {
        Err("Input simulation is only supported on Windows".into())
    }

    pub fn key_up(_key: &str) -> Result<(), String> {
        Err("Input simulation is only supported on Windows".into())
    }

    pub fn supports_key(_key: &str) -> bool {
        false
    }

    pub fn supported_key_names() -> Vec<String> {
        Vec::new()
    }

    pub fn left_click_at(_x: i32, _y: i32, _timing: ClickTiming) -> Result<(), String> {
        Err("Mouse automation is only supported on Windows".into())
    }

    pub fn move_cursor_to(_x: i32, _y: i32) -> Result<(), String> {
        Err("Mouse automation is only supported on Windows".into())
    }

    pub fn cursor_position() -> Result<(i32, i32), String> {
        Err("Mouse automation is only supported on Windows".into())
    }
}

pub use windows_input::*;

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn supports_common_aliases() {
        assert!(supports_key("SPACE"));
        assert!(supports_key("Spacebar"));
        assert!(supports_key("RIGHT CTRL"));
        assert!(supports_key("LEFT CTRL"));
        assert!(supports_key("RCTRL"));
        assert!(supports_key("MOUSE 4"));
        assert!(supports_key("MOUSE 5"));
        assert!(supports_key("XBUTTON1"));
        assert!(supports_key("XBUTTON2"));
    }
}
