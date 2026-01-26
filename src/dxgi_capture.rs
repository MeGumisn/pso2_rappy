use std::ffi::{c_void, CString};
use std::mem;
use crate::capture_settings::CapturePos;
use log::info;
use opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT;
use opencv::core::{CV_8UC3, CV_8UC4, Mat, MatTrait};
use opencv::imgproc;
use opencv::imgproc::cvt_color;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;

use memory_module_sys::{MemoryGetProcAddress, MemoryLoadLibrary, HMEMORYMODULE};
use windows::core::{Error, HRESULT};

// 初始化dxgi
type InitDxgiFn = unsafe extern "C" fn(hwnd: HWND);
// 抓取窗口
type GrabFn = unsafe extern "C" fn(
    buffer: *mut u8,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
) -> *mut u8;
// 销毁dxgi
type DestroyFn = unsafe extern "C" fn();

pub struct DxgiCapture {
    // 使用 'static 强转（仅当你能确保 lib 在结构体中一直存在时）
    _init_dxgi: InitDxgiFn,
    _grab: GrabFn,
    _destroy: DestroyFn,
    // 必须先声明 lib，确保它在 Symbol 之后被销毁
    _lib: HMEMORYMODULE,
    hwnd: HWND,
}

impl DxgiCapture {
    pub fn new(hwnd: HWND) -> Result<Self, Error> {
        unsafe {
            info!("Set process dpi aware.");
            let _ = SetProcessDPIAware();
            let dll_bytes = include_bytes!("../libs/dxgi4py.dll");
            let handle = MemoryLoadLibrary(dll_bytes.as_ptr() as *const c_void, dll_bytes.len());
            if handle.is_null() {
                return Err(Error::new(HRESULT(0), "Failed to load dxgi4py.dll"));
            }
            let init_dxgi_addr = Self::find_function_addr_in_dll_module(&handle, "init_dxgi")?;
            let init_dxgi:InitDxgiFn = mem::transmute(init_dxgi_addr);

            let grab_addr = Self::find_function_addr_in_dll_module(&handle, "grab")?;
            let grab:GrabFn = mem::transmute(grab_addr);

            let destroy_addr = Self::find_function_addr_in_dll_module(&handle, "destroy")?;
            let destroy:DestroyFn = mem::transmute(destroy_addr);


            // let lib = Library::new(dll_path)?;
            // let init_dxgi = std::mem::transmute::<Symbol<InitDxgiFn>, Symbol<'static, InitDxgiFn>>(
            //     lib.get_symbol_by_name("init_dxgi").unwrap().assume_type(),
            // );
            // let grab = std::mem::transmute::<Symbol<GrabFn>, Symbol<'static, GrabFn>>(
            //     lib.get_symbol_by_name("grab").unwrap().assume_type(),
            // );
            // let destroy = std::mem::transmute::<Symbol<DestroyFn>, Symbol<'static, DestroyFn>>(
            //     lib.get_symbol_by_name("destroy").unwrap().assume_type(),
            // );
            info!("Dxgi library loaded, start init_dxgi");
            init_dxgi(hwnd);
            info!("init_dxgi completed");
            Ok(Self {
                _init_dxgi: init_dxgi,
                _grab: grab,
                _destroy: destroy,
                _lib: handle,
                hwnd: hwnd,
            })
        }
    }

    fn find_function_addr_in_dll_module(
        module: &HMEMORYMODULE,
        func_name: &str,
    ) -> Result<*mut c_void, Error> {
        let c_func_name = CString::new(func_name).unwrap();
        let addr = unsafe { MemoryGetProcAddress(*module, c_func_name.as_ptr()) };
        if addr.is_null() {
            Err(Error::new(
                HRESULT(0),
                format!("Failed to get function {} address", func_name),
            ))
        } else {
            Ok(addr as *mut c_void)
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

#[cfg(test)]
pub fn show_image(display_mat: &Mat) {
    use opencv::highgui;
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
    use log::error;

    #[test]
    fn test_dxgi_capture() {
        let _logger = init_logger("debug");
        match search_window_by_title("PHANTASY") {
            Some(hwnd) => {
                let dxgi = DxgiCapture::new(hwnd).unwrap();
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
