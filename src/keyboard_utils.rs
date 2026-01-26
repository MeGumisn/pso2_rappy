use rand::random_range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_DOWN, VK_RETURN, VK_UP};
use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_KEYDOWN};

pub struct WindowsKeyboard {
    hwnd: HWND,
}

static IS_RUNNING: AtomicBool = AtomicBool::new(false);

impl WindowsKeyboard {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    fn repeat_send(&self, times: u16, msg: u32, vk: usize) {
        for _ in 0..times {
            unsafe { SendMessageW(self.hwnd, msg, Some(WPARAM(vk)), Some(LPARAM(0isize))) };
            sleep(Duration::from_millis(random_range(50..=100)));
        }
    }

    pub fn increase_rappy_coin(&self, num: u16) {
        self.repeat_send(num, WM_KEYDOWN, VK_UP.0 as usize);
    }

    pub fn decrease_rappy_coin(&self, num: u16) {
        self.repeat_send(num, WM_KEYDOWN, VK_DOWN.0 as usize);
    }

    pub fn play_rappy(&self) {
        unsafe {
            SendMessageW(
                self.hwnd,
                WM_KEYDOWN,
                Some(WPARAM(VK_RETURN.0 as usize)),
                Some(LPARAM(0isize)),
            );
        }
        sleep(Duration::from_millis(random_range(50..=100)));
    }

/*    pub fn is_scroll_lock_on() -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
        const VK_SCROLL: i16 = 0x91;
        unsafe { (GetKeyState(VK_SCROLL as i32) & 0x0001) != 0 }
    }*/

    pub fn state() -> bool {
        IS_RUNNING.load(Ordering::Relaxed)
    }

    pub fn stop_app() {
        IS_RUNNING.store(false, Ordering::Relaxed);
    }

    pub fn start_app() {
        IS_RUNNING.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows_utils::find_window_by_title;

    #[test]
    fn test_windows_keyboard() {
        if let Some(hwnd) = find_window_by_title("PHANTASY STAR ONLINE 2 NEW GENESIS") {
            let keyboard = WindowsKeyboard::new(hwnd);
            keyboard.increase_rappy_coin(5);
            keyboard.decrease_rappy_coin(4);
            keyboard.play_rappy();
        }
    }
}
