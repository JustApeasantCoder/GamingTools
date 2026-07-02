use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForegroundApp {
    pub executable: String,
    pub path: String,
}

#[cfg(windows)]
pub fn current_app() -> Result<ForegroundApp, String> {
    use std::path::Path;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };

    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return Err("No foreground application is available".into());
    }

    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    if process_id == 0 {
        return Err("Could not identify the foreground application".into());
    }

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return Err(format!(
            "Could not inspect the foreground application: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut buffer = vec![0u16; 32_768];
    let mut length = buffer.len() as u32;
    let queried =
        unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
    unsafe { CloseHandle(process) };
    if queried == 0 {
        return Err(format!(
            "Could not read the foreground application path: {}",
            std::io::Error::last_os_error()
        ));
    }

    let path = String::from_utf16_lossy(&buffer[..length as usize]);
    let executable = Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_string();
    Ok(ForegroundApp { executable, path })
}

#[cfg(not(windows))]
pub fn current_app() -> Result<ForegroundApp, String> {
    Err("Foreground application detection is only supported on Windows".into())
}

pub fn matches_executable(expected: &str) -> bool {
    if expected.trim().is_empty() {
        return true;
    }
    current_app()
        .map(|app| app.executable.eq_ignore_ascii_case(expected.trim()))
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn focus_executable(expected: &str) -> Result<(), String> {
    use std::path::Path;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, BOOL, HWND, LPARAM},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow,
        },
    };

    if expected.trim().is_empty() {
        return Ok(());
    }

    struct Search {
        expected: String,
        matched: HWND,
    }

    unsafe extern "system" fn visit_window(window: HWND, lparam: LPARAM) -> BOOL {
        if unsafe { IsWindowVisible(window) } == 0 {
            return 1;
        }

        let search = unsafe { &mut *(lparam as *mut Search) };
        let mut process_id = 0;
        unsafe { GetWindowThreadProcessId(window, &mut process_id) };
        if process_id == 0 {
            return 1;
        }

        if process_matches(process_id, &search.expected) {
            search.matched = window;
            return 0;
        }
        1
    }

    fn process_matches(process_id: u32, expected: &str) -> bool {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return false;
        }
        let mut buffer = vec![0u16; 32_768];
        let mut length = buffer.len() as u32;
        let queried =
            unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) };
        unsafe { CloseHandle(process) };
        if queried == 0 {
            return false;
        }

        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        let executable = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&path);
        executable.eq_ignore_ascii_case(expected.trim())
            || path.eq_ignore_ascii_case(expected.trim())
    }

    let mut search = Search {
        expected: expected.trim().to_string(),
        matched: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(Some(visit_window), &mut search as *mut Search as LPARAM);
    }
    if search.matched.is_null() {
        return Err(format!(
            "Could not find a visible window for {}",
            expected.trim()
        ));
    }
    let focused = unsafe { SetForegroundWindow(search.matched) };
    if focused == 0 {
        Err(format!(
            "Could not focus {}: {}",
            expected.trim(),
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn focus_executable(_expected: &str) -> Result<(), String> {
    Err("Foreground focusing is only supported on Windows".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_guard_is_unrestricted() {
        assert!(super::matches_executable(""));
    }
}
