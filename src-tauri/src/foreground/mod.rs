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

#[cfg(test)]
mod tests {
    #[test]
    fn empty_guard_is_unrestricted() {
        assert!(super::matches_executable(""));
    }
}
