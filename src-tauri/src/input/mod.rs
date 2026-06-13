#[cfg(windows)]
mod windows_input {
    use std::{mem::size_of, thread, time::Duration};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
        MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
        VK_CONTROL, VK_ESCAPE, VK_LBUTTON, VK_MBUTTON, VK_MENU, VK_RBUTTON, VK_RETURN, VK_SHIFT,
        VK_SPACE, VK_TAB, VK_XBUTTON1, VK_XBUTTON2,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;

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
        unsafe {
            if SetCursorPos(x, y) == 0 {
                return Err("Unable to move cursor to inventory slot".into());
            }
        }
        thread::sleep(Duration::from_millis(timing.cursor_settle_ms));

        if let Err(error) = send_down("LEFT CLICK") {
            return Err(error);
        }
        thread::sleep(Duration::from_millis(timing.click_hold_ms));
        let result = send_up("LEFT CLICK");
        if result.is_err() {
            let _ = send_up("LEFT CLICK");
        }
        thread::sleep(Duration::from_millis(timing.click_release_settle_ms));
        result
    }

    fn send_down(key: &str) -> Result<(), String> {
        match input_code(key).ok_or_else(|| format!("Unsupported key: {key}"))? {
            InputCode::Keyboard(vk) => send_keyboard(vk, false),
            InputCode::Mouse {
                down, mouse_data, ..
            } => send_mouse(down, mouse_data),
        }
    }

    fn send_up(key: &str) -> Result<(), String> {
        match input_code(key).ok_or_else(|| format!("Unsupported key: {key}"))? {
            InputCode::Keyboard(vk) => send_keyboard(vk, true),
            InputCode::Mouse { up, mouse_data, .. } => send_mouse(up, mouse_data),
        }
    }

    fn send_keyboard(vk: u16, key_up: bool) -> Result<(), String> {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
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
        Keyboard(u16),
        Mouse {
            vk: u16,
            down: u32,
            up: u32,
            mouse_data: u32,
        },
    }

    fn virtual_key_code(key: &str) -> Option<u16> {
        match input_code(key)? {
            InputCode::Keyboard(vk) => Some(vk),
            InputCode::Mouse { vk, .. } => Some(vk),
        }
    }

    fn input_code(key: &str) -> Option<InputCode> {
        let normalized = key.trim().to_uppercase();
        if normalized.len() == 1 {
            let byte = normalized.as_bytes()[0];
            if byte.is_ascii_alphanumeric() {
                return Some(InputCode::Keyboard(byte as u16));
            }
        }

        match normalized.as_str() {
            "F1" => Some(InputCode::Keyboard(0x70)),
            "F2" => Some(InputCode::Keyboard(0x71)),
            "F3" => Some(InputCode::Keyboard(0x72)),
            "F4" => Some(InputCode::Keyboard(0x73)),
            "F5" => Some(InputCode::Keyboard(0x74)),
            "F6" => Some(InputCode::Keyboard(0x75)),
            "F7" => Some(InputCode::Keyboard(0x76)),
            "F8" => Some(InputCode::Keyboard(0x77)),
            "F9" => Some(InputCode::Keyboard(0x78)),
            "F10" => Some(InputCode::Keyboard(0x79)),
            "F11" => Some(InputCode::Keyboard(0x7A)),
            "F12" => Some(InputCode::Keyboard(0x7B)),
            "SPACE" | "SPACEBAR" => Some(InputCode::Keyboard(VK_SPACE)),
            "ENTER" => Some(InputCode::Keyboard(VK_RETURN)),
            "ESC" | "ESCAPE" => Some(InputCode::Keyboard(VK_ESCAPE)),
            "TAB" => Some(InputCode::Keyboard(VK_TAB)),
            "SHIFT" => Some(InputCode::Keyboard(VK_SHIFT)),
            "CTRL" | "CONTROL" => Some(InputCode::Keyboard(VK_CONTROL)),
            "ALT" => Some(InputCode::Keyboard(VK_MENU)),
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
}

pub use windows_input::*;

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn supports_common_aliases() {
        assert!(supports_key("SPACE"));
        assert!(supports_key("Spacebar"));
        assert!(supports_key("MOUSE 4"));
        assert!(supports_key("MOUSE 5"));
        assert!(supports_key("XBUTTON1"));
        assert!(supports_key("XBUTTON2"));
    }
}
