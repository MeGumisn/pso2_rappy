use crate::capture_settings::CapturePos;
use libloading::{Error, Library, Symbol};
use log::info;
use opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT;
use opencv::core::{
    CV_8UC3, CV_8UC4, Mat, MatTrait,
};
use opencv::{highgui, imgproc};
use opencv::imgproc::cvt_color;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;

// 初始化dxgi
type InitDxgiFn = unsafe extern "system" fn(hwnd: HWND);
// 抓取窗口
type GrabFn = unsafe extern "system" fn(
    buffer: *mut u8,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> *mut u8;
// 销毁dxgi
type DestroyFn = unsafe extern "system" fn();

pub struct DxgiCapture {
    // 使用 'static 强转（仅当你能确保 lib 在结构体中一直存在时）
    _init_dxgi: Symbol<'static, InitDxgiFn>,
    _grab: Symbol<'static, GrabFn>,
    _destroy: Symbol<'static, DestroyFn>,
    // 必须先声明 lib，确保它在 Symbol 之后被销毁
    _lib: Library,
    hwnd: HWND,
}

impl DxgiCapture {
    pub fn new(dll_path: &str, hwnd: HWND) -> Result<Self, Error> {
        unsafe {
            info!("Set process dpi aware.");
            let _ = SetProcessDPIAware();
            let lib = Library::new(dll_path)?;
            let init_dxgi = std::mem::transmute::<Symbol<InitDxgiFn>, Symbol<'static, InitDxgiFn>>(
                lib.get(b"init_dxgi")?,
            );
            let grab =
                std::mem::transmute::<Symbol<GrabFn>, Symbol<'static, GrabFn>>(lib.get(b"grab")?);
            let destroy = std::mem::transmute::<Symbol<DestroyFn>, Symbol<'static, DestroyFn>>(
                lib.get(b"destroy")?,
            );
            info!("Dxgi library loaded, start init_dxgi");
            init_dxgi(hwnd);
            info!("init_dxgi completed");
            Ok(Self {
                _init_dxgi: init_dxgi,
                _grab: grab,
                _destroy: destroy,
                _lib: lib,
                hwnd: hwnd,
            })
        }
    }

    pub fn grab(&self, pos: &CapturePos) -> Mat {
        let (left, top, width, height) = pos.rect;
        let mat = self._grab(left, top, width, height);
        let mut display_mat = unsafe { Mat::new_rows_cols(height, width, CV_8UC3) }.unwrap();
        let _ = cvt_color(
            &mat,
            &mut display_mat,
            imgproc::COLOR_BGRA2BGR,
            0,
            ALGO_HINT_DEFAULT,
        );
        display_mat
    }

    pub fn grab_gray(&self, pos: &CapturePos) -> Mat {
        let (left, top, width, height) = pos.rect;
        let mat = self._grab(left, top, width, height);
        let mut display_mat = unsafe { Mat::new_rows_cols(height, width, CV_8UC3) }.unwrap();
        let _ = cvt_color(
            &mat,
            &mut display_mat,
            imgproc::COLOR_BGRA2GRAY,
            0,
            ALGO_HINT_DEFAULT,
        );
        display_mat
    }

    pub fn update_hwnd(&mut self, hwnd: HWND) {
        self.hwnd = hwnd;
        unsafe {
            (self._destroy)();
            (self._init_dxgi)(hwnd);
        }
    }

    fn _grab(&self, left: i32, top: i32, width: i32, height: i32) -> Mat {
        let mut mat;
        unsafe {
            mat = Mat::new_rows_cols(height, width, CV_8UC4).unwrap();
            let _ = (self._grab)(mat.data_mut(), left, top, width, height);
        }
        mat
    }
}

impl Drop for DxgiCapture {
    fn drop(&mut self) {
        unsafe { (self._destroy)() }
    }
}

pub fn show_image(display_mat: &Mat){
    // 创建窗口并显示
    let window_name = "Rust OpenCV Display";
    highgui::named_window(window_name, highgui::WINDOW_AUTOSIZE).unwrap();
    highgui::imshow(window_name, display_mat).unwrap();

    // 等待按键（否则窗口会闪现即逝）
    highgui::wait_key(0).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::init_logger;
    use crate::windows_utils::search_window_by_title;
    use log::{error, LevelFilter};

    #[test]
    fn test_dxgi_capture() {
        let _logger = init_logger("debug");
        match search_window_by_title("PHANTASY") {
            Some(hwnd) => {
                let dxgi = DxgiCapture::new("libs/dxgi4py.dll", hwnd).unwrap();
                // 假设有一个有效的HWND
                let display_mat = dxgi.grab(&CapturePos {
                    rect: (0, 0, 1600, 954),
                });
                show_image(&display_mat);
            }
            _ => {
                error!("Window not found");
            }
        }
    }
}
