use log::{error, info};
use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, LPARAM, POINT};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, FindWindowW, GetWindowTextW};
use windows::core::{BOOL, HSTRING, PCWSTR};
///
/// * 根据完整窗口名获取HWND
/// * 注意: 如果出现坐标不对的情况, 需要看下SetProcessDPIAware是否调用
///
pub fn find_window_by_title(window_name: &str) -> Option<HWND> {
    let title = HSTRING::from(window_name);
    unsafe {
        match FindWindowW(None, PCWSTR::from_raw(title.as_ptr())) {
            Ok(handle) => Some(handle),
            Err(e) => {
                error!(
                    "FindWindowByTitle error: {:?}, window name: {}",
                    e, window_name
                );
                None
            }
        }
    }
}

struct WindowFinder {
    title: String,
    found_hwnd: Option<HWND>,
}

extern "system" fn window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Here you would implement the logic to check the window title
    // and compare it with the desired title passed via lparam.
    // If a match is found, you can store the HWND in a location
    // accessible via lparam.
    let window_finder = unsafe { &mut *(lparam.0 as *mut WindowFinder) };
    let mut buffer: [u16; 256] = [0; 256];
    unsafe {
        GetWindowTextW(hwnd, &mut buffer);
    }
    let window_title = String::from_utf16_lossy(&buffer);
    if window_title.contains(&window_finder.title) {
        window_finder.found_hwnd = Some(hwnd);
        // 已经找到窗口, 返回false终止枚举
        BOOL(0)
    } else {
        // 未找到, 返回true继续枚举
        BOOL(1)
    }
}

/// 遍历所有窗口，查找匹配的窗口名，返回HWND
pub fn search_window_by_title(window_name: &str) -> Option<HWND> {
    let mut finder = WindowFinder {
        title: window_name.to_string(),
        found_hwnd: None,
    };
    unsafe {
        let _ = EnumWindows(
            Some(window_callback),
            LPARAM(&mut finder as *mut _ as isize),
        );
    }
    finder.found_hwnd
}

pub fn update_window(window_title: &str) -> Option<HWND> {
    if let Some(hwnd) = find_window_by_title(window_title) {
        info!("Try updating window {}", window_title);
        unsafe {
            let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, true);
            let _ = windows::Win32::Graphics::Gdi::UpdateWindow(hwnd);
        }
        Some(hwnd)
    } else {
        None
    }
}

pub fn get_window_client_offset(hwnd: HWND) -> Option<(i32, i32)> {
    use windows::Win32::Foundation::RECT;
    let mut rect = RECT::default();
    unsafe {
        match DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut RECT as *mut c_void,
            size_of::<RECT>() as u32,
        ) {
            Ok(_) => {
                let mut point = POINT::default();
                if ClientToScreen(hwnd, &mut point).as_bool() {
                    let offset_x = point.x - rect.left;
                    let offset_y = point.y - rect.top;
                    Some((offset_x, offset_y))
                } else {
                    error!("ClientToScreen failed for hwnd: {:?}", hwnd);
                    None
                }
            }
            Err(e) => {
                error!("DwmGetWindowAttribute error: {:?}", e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging;
    use log::info;
    use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;

    #[test]
    fn test_find_window_by_title() {
        let _logger = logging::init_logger("debug");
        // This test assumes that there is a window with the title "Untitled - Notepad"
        // Make sure to open Notepad with that title before running the test
        if let Some(hwnd) = find_window_by_title("PHANTASY STAR ONLINE 2 NEW GENESIS") {
            info!("hwnd: {:?}", hwnd);
            assert!(!hwnd.is_invalid(), "Window handle should not be invalid");
        }
    }

    #[test]
    fn test_search_window_by_title() {
        let _logger = logging::init_logger("debug");
        // This test assumes that there is a window with the title "Untitled - Notepad"
        // Make sure to open Notepad with that title before running the test
        let hwnd_option = search_window_by_title("PHANTASY STAR ONLINE 2");
        match hwnd_option {
            Some(hwnd) => {
                info!("Found hwnd: {:?}", hwnd);
                assert!(!hwnd.is_invalid(), "Window handle should not be invalid");
            }
            None => {
                error!("Window not found");
            }
        }
    }

    #[test]
    fn test_get_window_client_offset() {
        let _logger = logging::init_logger("debug");
        let _ = unsafe { SetProcessDPIAware() };
        if let Some(hwnd) = search_window_by_title("PHANTASY STAR ONLINE 2") {
            if let Some((offset_x, offset_y)) = get_window_client_offset(hwnd) {
                info!("Client offset: ({}, {})", offset_x, offset_y);
            } else {
                error!("Failed to get client offset");
            }
        } else {
            error!("Window not found");
        }
    }
}
