use log::error;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, FindWindowW, GetWindowTextW};
use windows::core::{BOOL, HSTRING, PCWSTR};

/// 根据完整窗口名获取HWND
pub fn find_window_by_title(window_name: &str) -> HWND {
    let title = HSTRING::from(window_name);
    unsafe {
        match FindWindowW(None, PCWSTR::from_raw(title.as_ptr())) {
            Ok(handle) => handle,
            Err(e) => {
                error!(
                    "FindWindowByTitle error: {:?}, window name: {}",
                    e,
                    window_name
                );
                panic!("Failed to find window: {}", e);
            }
        }
    }
}

struct WindowFinder{
    title: String,
    found_hwnd: Option<HWND>,
}

extern "system" fn window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Here you would implement the logic to check the window title
    // and compare it with the desired title passed via lparam.
    // If a match is found, you can store the HWND in a location
    // accessible via lparam.
    let window_finder =  unsafe {&mut *(lparam.0 as *mut WindowFinder) };
    let mut buffer:[u16;256] = [0;256];
    unsafe {
        GetWindowTextW(hwnd, &mut buffer);
    }
    let window_title = String::from_utf16_lossy(&buffer);
    if window_title.contains(&window_finder.title){
        window_finder.found_hwnd = Some(hwnd);
        // 已经找到窗口, 返回false终止枚举
        BOOL(0)
    }else{
        // 未找到, 返回true继续枚举
        BOOL(1)
    }

}

/// 遍历所有窗口，查找匹配的窗口名，返回HWND
pub  fn  search_window_by_title(window_name: &str) -> Option<HWND> {
    let mut finder = WindowFinder{
        title: window_name.to_string(),
        found_hwnd: None,
    };
    unsafe {
        let _ = EnumWindows(Some(window_callback), LPARAM(&mut finder as * mut _ as isize));
    }
    finder.found_hwnd
}

#[cfg(test)]
mod tests {
    use log::info;
    use crate::logging;
    use super::*;
    #[test]
    fn test_find_window_by_title() {
        logging::init_logger();
        // This test assumes that there is a window with the title "Untitled - Notepad"
        // Make sure to open Notepad with that title before running the test
        let hwnd = find_window_by_title("PHANTASY STAR ONLINE 2 NEW GENESIS");
        info!("hwnd: {:?}", hwnd);
        assert!(!hwnd.is_invalid(), "Window handle should not be invalid");
    }

    #[test]
    fn test_search_window_by_title() {
        logging::init_logger();
        // This test assumes that there is a window with the title "Untitled - Notepad"
        // Make sure to open Notepad with that title before running the test
        let hwnd_option = search_window_by_title("PHANTASY STAR ONLINE 2");
        match hwnd_option {
            Some(hwnd) => {
                info!("Found hwnd: {:?}", hwnd);
                assert!(!hwnd.is_invalid(), "Window handle should not be invalid");
            },
            None => {
                error!("Window not found");
            }
        }
    }
}
