use libloading::{Error, Library, Symbol};
// 初始化dxgi
type InitDxgiFn = unsafe extern "system" fn(hwnd: windows::Win32::Foundation::HWND);
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
    // 必须先声明 lib，确保它在 Symbol 之后被销毁
    _lib: Library,
    // 使用 'static 强转（仅当你能确保 lib 在结构体中一直存在时）
    _init_dxgi: Symbol<'static, InitDxgiFn>,
    _grab: Symbol<'static, GrabFn>,
    _destroy: Symbol<'static, DestroyFn>,
}

impl DxgiCapture {
    pub fn new(dll_path: &str) -> Result<Self, Error> {
        unsafe {
            let lib = Library::new(dll_path)?;
            let init_dxgi = std::mem::transmute::<Symbol<InitDxgiFn>, Symbol<'static, InitDxgiFn>>(
                lib.get(b"init_dxgi")?,
            );
            let grab =
                std::mem::transmute::<Symbol<GrabFn>, Symbol<'static, GrabFn>>(lib.get(b"grab")?);
            let destroy = std::mem::transmute::<Symbol<DestroyFn>, Symbol<'static, DestroyFn>>(
                lib.get(b"destroy")?,
            );
            Ok(Self {
                _lib: lib,
                _init_dxgi: init_dxgi,
                _grab: grab,
                _destroy: destroy,
            })
        }
    }

    pub fn init_dxgi(&self, hwnd: windows::Win32::Foundation::HWND) {
        unsafe {
            (self._init_dxgi)(hwnd);
        }
    }

    pub fn grab(&self, left: i32, top: i32, width: i32, height: i32) -> Vec<u8> {
        let buffer_size = (width * height * 4) as usize; // Assuming 4 bytes per pixel (RGBA)
        let buffer = vec![0u8; buffer_size];
        unsafe {
            let _ = (self._grab)(buffer.as_ptr().cast_mut(), left, top, width, height);
            buffer
        }
    }

    fn drop(&self) {
        unsafe { (self._destroy)() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::init_logger;
    use crate::windows_utils::search_window_by_title;
    use log::error;
    use opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT;
    use opencv::core::{Mat, Mat_AUTO_STEP, Size, CV_8UC3, CV_8UC4};
    use opencv::imgproc::{cvt_color};
    use std::ffi::c_void;
    use opencv::{highgui, imgproc};
    use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;

    #[test]
    fn test_dxgi_capture() {
        init_logger();
        let dxgi = DxgiCapture::new("libs/dxgi4py.dll").unwrap();
        // 假设有一个有效的HWND
        match search_window_by_title("PHANTASY STAR ONLINE 2 NEW GENESIS") {
            Some(hwnd) => {
                unsafe {
                    // 系统DPI感知
                    let _ = SetProcessDPIAware();
                    dxgi.init_dxgi(hwnd);
                    let (width, height) = (512, 512);
                    let buffer = dxgi.grab(0, 0, width, height);
                    let mat = Mat::new_size_with_data_unsafe(
                        Size::new(width, height),
                        CV_8UC4,
                        buffer.as_ptr() as *mut c_void,
                        Mat_AUTO_STEP,
                    ).unwrap();
                    let mut display_mat = Mat::new_size(Size::new(width,height),CV_8UC3).unwrap();
                    let _ = cvt_color(&mat, &mut display_mat, imgproc::COLOR_BGRA2BGR,0,ALGO_HINT_DEFAULT);

                    // 创建窗口并显示
                    let window_name = "Rust OpenCV Display";
                    highgui::named_window(window_name, highgui::WINDOW_AUTOSIZE).unwrap();
                    highgui::imshow(window_name, &display_mat).unwrap();

                    // 等待按键（否则窗口会闪现即逝）
                    highgui::wait_key(0).unwrap();
                }
                dxgi.drop();
            }
            _ => {
                error!("Window not found");
            }
        }
    }
}
