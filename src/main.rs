use crate::capture_settings::CapturePos;
use libloading::Error;
use log::{debug, error, info};
use opencv::core::{Mat, MatTraitConst, Size, Vector, min_max_loc, no_array};
use opencv::imgcodecs::imwrite;
use opencv::imgproc::{INTER_LINEAR, TM_CCORR_NORMED, match_template, resize};
use std::thread::sleep;
use std::time::Duration;

mod capture_settings;
mod dxgi_capture;
mod keyboard_utils;
mod logging;
mod rappy_checker;
mod template_img;
mod windows_utils;
use crate::dxgi_capture::DxgiCapture;
use crate::keyboard_utils::WindowsKeyboard;
use crate::rappy_checker::get_threshold_mat;
use crate::template_img::TemplateImg;
use crate::windows_utils::{search_window_by_title, update_window};

static QTE_DIR: &str = "QTE_";
static TARGET_DIR: &str = "TARGET_";

static IMG_THRESH: u8 = 190;

fn check_or_create_dir(path: &str) {
    use std::fs;
    use std::path::Path;
    let dir_path = Path::new(path);
    if !dir_path.exists() || dir_path.is_file() {
        if let Err(e) = fs::create_dir_all(dir_path) {
            error!("Failed to create directory {}: {}", path, e);
        }
    } else {
        info!("Directory {} already exists.", path);
    }
}

///
/// 检查游戏中的截图(shot)部分是否和模板(TemplateImg)中的图片相似
///
/// # Examples
///
/// ```
///    match search_window_by_title("PHANTASY STAR ONLINE 2"){
///         Some(hwnd)=>{
///             let capture = DxgiCapture::new("libs/dxgi.dll", hwnd)?;
///             let is_similar = check_game_shot(&capture, &CapturePos::KEY_READY, TemplateImg::KEY_READY.deref(), 0.9);
///             info!("Similar key_ready_shot: {}", is_similar);
///             Ok(())
///         },
///         _=>{
///             info!("No hwnd found");
///             Ok(())
///         }
///     }
/// ```
/// ## Overloaded Parameters
///
/// ## Parameters
///
/// * capture: the dxgi capture.
/// * capture_pos: the position of game shot, this is a relative position for (game) window.
/// * template_img: the image decode from png code(created by opencv im_encode)
/// * sim_threshold: the minimum value of SSIM similarity.
///
/// ## Return
///
/// * return true if the SSIM similarity > sim_threshold.
fn check_game_shot(
    capture: &DxgiCapture,
    capture_pos: &CapturePos,
    template_img: &TemplateImg,
    sim_threshold: f64,
    threshed: bool,
) -> bool {
    if threshed {
        let game_shot = capture.grab(capture_pos);
        if cfg!(debug_assertions) {
            // show_image(&game_shot);
        }
        let game_shot = get_threshold_mat(&game_shot, IMG_THRESH);
        let img = get_threshold_mat(&template_img.img, IMG_THRESH);

        let is_gray = game_shot.channels() == 1;
        let sim = rappy_checker::get_ssim(&game_shot, &img, is_gray).unwrap();
        info!("Sim: {}", sim);
        sim > sim_threshold
    } else {
        let game_shot = capture.grab_gray(capture_pos);
        let img = &template_img.img;
        let is_gray = game_shot.channels() == 1;
        let sim = rappy_checker::get_ssim(&game_shot, img, is_gray).unwrap();
        info!("Sim: {}", sim);
        sim > sim_threshold
    }
}

fn check_qte_appear(capture: &DxgiCapture) -> bool {
    if cfg!(debug_assertions) {
        // show_image(&game_shot);
        debug!("Checking QTE appear...");
    }
    let rappy_qte_shot = capture.grab_gray(&CapturePos::QTE);
    let mut resized_rappy_qte_shot = Mat::default();
    resize(
        &rappy_qte_shot,
        &mut resized_rappy_qte_shot,
        Size::new(0, 0),
        0.5,
        0.5,
        INTER_LINEAR,
    )
    .unwrap();
    let mut res_mat = Mat::default();
    match_template(
        &resized_rappy_qte_shot,
        &TemplateImg::QTE.img,
        &mut res_mat,
        TM_CCORR_NORMED,
        &no_array(),
    )
    .unwrap();
    let mut max_val = 0f64;
    min_max_loc(&res_mat, None, Some(&mut max_val), None, None, &no_array()).unwrap();
    if max_val > 0.99 {
        // 生成时间戳文件名
        let png_name = chrono::Local::now().format("%Y%m%d%H%M%S%.6f").to_string();
        info!("qte name:{}, sim: {:.6}", png_name, max_val);
        // 确保目录存在 (Rust 不会自动创建目录，需使用 std::fs::create_dir_all)
        let file_path = format!("{}/{}.png", QTE_DIR, png_name);
        // 保存图片 (params 传空 Vector)
        info!("QTE image name: {}, sim: {}.", file_path, max_val);
        imwrite(&file_path, &resized_rappy_qte_shot, &Vector::new()).unwrap();
        return true;
    }
    false
}

fn wait_for_key_ready(capture: &DxgiCapture) {
        info!("Waiting for key ready...");
    // scroll灯亮起但游戏中开始(回车键)不可用，等待
    while !check_game_shot(
        capture,
        &CapturePos::KEY_READY,
        &TemplateImg::KEY_READY,
        0.9,
        true,
    ) && WindowsKeyboard::is_scroll_lock_on()
    {
        sleep(Duration::from_millis(2000));
    }
}

fn try_increase_coin_while_energy_is_four(
    capture: &DxgiCapture,
    keyboard: &WindowsKeyboard,
    bet_coin_is_one: &mut bool,
    burst: &mut bool,
) {
        info!("Trying to increase coin for the keyboard.");
    if *bet_coin_is_one
        && check_game_shot(
            capture,
            &CapturePos::ENERGY_FOUR,
            &TemplateImg::ENERGY_FOUR,
            0.85,
            true,
        )
    {
        info!("Bet coin = 1,  energy = 4, increase bet coin.");
        // 没到5枚硬币时连续按上键(最大20次)
        for i in 0..=20 {
            if !check_game_shot(
                &capture,
                &CapturePos::COIN_COUNT,
                &TemplateImg::COIN_FIVE,
                0.85,
                true,
            ) {
                keyboard.increase_rappy_coin(1);
            } else {
                // 按键20次依然没有增加到5枚硬币,说明卡在pse页面
                if i == 20 {
                    *burst = true;
                }
                break;
            }
        }
        // 已经执行增加coin操作, bet_coin_one设置为false
        *bet_coin_is_one = false;
    }
}

fn try_decrease_coin_while_energy_is_zero(
    capture: &DxgiCapture,
    keyboard: &WindowsKeyboard,
    bet_coin_is_one: &mut bool,
) {
        info!("Trying to decrease coin for the keyboard.");
    if !*bet_coin_is_one
        && check_game_shot(
            &capture,
            &CapturePos::ENERGY_ZERO,
            &TemplateImg::ENERGY_ZERO,
            0.85,
            true,
        )
    {
        info!("Bet coin > 1,  energy = 0, decrease bet coin.");
        for _i in 0..=20 {
            if !check_game_shot(
                &capture,
                &CapturePos::COIN_COUNT,
                &TemplateImg::COIN_ONE,
                0.85,
                true,
            ) {
                keyboard.decrease_rappy_coin(1);
            }
        }
        *bet_coin_is_one = false;
    }
}

fn process_rappy_qte(capture: &DxgiCapture, burst: &mut bool) {
        info!("Processing rappy qte...");
    if check_game_shot(
        capture,
        &CapturePos::TARGET,
        &TemplateImg::TARGET,
        0.7,
        true,
    ) || *burst
    {
        info!("Rappy target appear, wait for QTE.");
        // 等qte完全开始
        sleep(Duration::from_millis(3000));
        while !check_qte_appear(capture) {
            continue;
        }
        info!("QTE appear, ready.");
        // 出现QTE, 排除掉rappy burst的可能
        *burst = false;
    }
}

fn main() -> Result<(), Error> {
    let _logger = logging::init_logger("info");
    check_or_create_dir(TARGET_DIR);
    check_or_create_dir(QTE_DIR);
    let window_name = "PHANTASY STAR ONLINE 2";
    match search_window_by_title(window_name) {
        Some(hwnd) => {
            let mut capture = DxgiCapture::new("libs/dxgi4py.dll", hwnd)?;
            // 检查赌场币是否为1
            let mut bet_coin_is_one = check_game_shot(
                &capture,
                &CapturePos::COIN_COUNT,
                &TemplateImg::COIN_ONE,
                0.85,
                true,
            );
            info!("Start, check bet coin nums == 1: {}", bet_coin_is_one);
            // pse burst状态, 初始为false
            let mut burst = false;
            let mut keyboard = WindowsKeyboard::new(hwnd);

            let mut sleep_time = 30 * 1000;
            loop {
                if WindowsKeyboard::is_scroll_lock_on() {
                    // 等待按下回车
                    wait_for_key_ready(&capture);
                    // 赌场币为1枚,能量为4格时,增加赌场币到5枚,等待满能量pse
                    try_increase_coin_while_energy_is_four(
                        &capture,
                        &keyboard,
                        &mut bet_coin_is_one,
                        &mut burst,
                    );
                    // 赌场币不为1枚，能力不足4格，将赌场币降低到1
                    try_decrease_coin_while_energy_is_zero(
                        &capture,
                        &keyboard,
                        &mut bet_coin_is_one,
                    );
                    process_rappy_qte(&capture, &mut burst);
                    if check_game_shot(
                        &capture,
                        &CapturePos::KEY_READY,
                        &TemplateImg::KEY_READY,
                        0.9,
                        true,
                    ) {
                        info!("Trying to press enter.");
                        keyboard.play_rappy();
                    }
                    if !check_game_shot(
                        &capture,
                        &CapturePos::COIN_COUNT,
                        &TemplateImg::COIN_ONE,
                        0.85,
                        true,
                    ) && !check_game_shot(
                        &capture,
                        &CapturePos::COIN_COUNT,
                        &TemplateImg::COIN_FIVE,
                        0.85,
                        true,
                    ) {
                        // 画面错位，刷新下这个窗口试下
                        if let Some(_hwnd) = update_window(&window_name) {
                            capture.update_hwnd(_hwnd);
                            keyboard = WindowsKeyboard::new(_hwnd);
                        }
                    }
                } else {
                    if sleep_time >= 30 {
                        info!("waiting until scroll lock is on....");
                        sleep_time = 0;
                    }
                    sleep(Duration::from_millis(1000));
                    sleep_time += 1;
                }
            }
        }
        None => {
            error!("Search window: {} failed", window_name);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxgi_capture::show_image;

    #[test]
    fn test_grab_and_check() -> Result<(), Error> {
        match search_window_by_title("PHANTASY STAR ONLINE 2") {
            Some(hwnd) => {
                let capture = DxgiCapture::new("libs/dxgi4py.dll", hwnd)?;
                info!("Check key ready");
                let template = TemplateImg::KEY_READY;
                let img = &template.img;
                info!(
                    "Template image channel: {}, rows: {}, cols: {}",
                    img.channels(),
                    img.rows(),
                    img.cols()
                );
                let is_similar =
                    check_game_shot(&capture, &CapturePos::KEY_READY, &template, 0.85, true);
                info!("Similar key_ready_shot: {}", is_similar);
                Ok(())
            }
            _ => {
                info!("No hwnd found");
                Ok(())
            }
        }
    }

    #[test]
    fn test_grab_coin_one() -> Result<(), Error> {
        match search_window_by_title("PHANTASY STAR ONLINE 2") {
            Some(hwnd) => {
                let capture = DxgiCapture::new("libs/dxgi4py.dll", hwnd)?;
                let coin_one = capture.grab(&CapturePos::COIN_COUNT);
                show_image(&coin_one);
                Ok(())
            }
            None => Ok(()),
        }
    }

    #[test]
    fn test_match_qte() -> Result<(), Error> {
        match search_window_by_title("PHANTASY STAR ONLINE 2") {
            Some(hwnd) => {
                let capture = DxgiCapture::new("libs/dxgi4py.dll", hwnd)?;
                check_qte_appear(&capture);
                Ok(())
            }
            None => Ok(()),
        }
    }
}
