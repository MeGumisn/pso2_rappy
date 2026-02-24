use crate::capture_settings::CapturePos;
use crate::dxgi_capture::DxgiCapture;
use crate::keyboard_utils::WindowsKeyboard;
use crate::rappy_checker;
use crate::rappy_checker::get_threshold_mat;
use crate::template_img::TemplateImg;
use crate::windows_utils::{get_window_client_offset, search_window_by_title, update_window};
use egui::Context;
use log::{debug, error, info};
use opencv::core::{Mat, MatTraitConst, Size, Vector, min_max_loc, no_array};
use opencv::imgcodecs::imwrite;
use opencv::imgproc::{INTER_LINEAR, TM_CCORR_NORMED, match_template, resize};
use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;
use windows::core::Error;

pub(crate) static QTE_DIR: &str = "QTE_";
pub(crate) static TARGET_DIR: &str = "TARGET_";

static IMG_THRESH: u8 = 190;

pub fn check_or_create_dir(path: &str) {
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
///             let is_similar = check_game_shot(&capture, &CapturePos::key_ready, TemplateImg::key_ready.deref(), 0.9);
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
        // info!("Sim: {}", sim);
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

struct AutoRappy {
    offset_x: i32,
    offset_y: i32,
}

impl AutoRappy {
    fn check_qte_appear(&self, capture: &DxgiCapture, tx: &Sender<String>) -> bool {
        if cfg!(debug_assertions) {
            // show_image(&game_shot);
            debug!("Checking qte appear...");
        }
        let rappy_qte_shot = capture.grab_gray(&CapturePos::qte(self.offset_x, self.offset_y));
        let mut resized_rappy_qte_shot = Mat::default();
        
        if resize(
            &rappy_qte_shot,
            &mut resized_rappy_qte_shot,
            Size::new(0, 0),
            0.5,
            0.5,
            INTER_LINEAR,
        ).is_err() {
            error!("Failed to resize QTE image");
            return false;
        }
        
        let mut res_mat = Mat::default();
        if match_template(
            &resized_rappy_qte_shot,
            &TemplateImg::QTE.img,
            &mut res_mat,
            TM_CCORR_NORMED,
            &no_array(),
        ).is_err() {
            error!("Failed to match template for QTE");
            return false;
        }
        
        let mut max_val = 0f64;
        if min_max_loc(&res_mat, None, Some(&mut max_val), None, None, &no_array()).is_err() {
            error!("Failed to find max value in match result");
            return false;
        }
        
        if max_val > 0.99 {
            // 生成时间戳文件名
            let png_name = chrono::Local::now().format("%Y%m%d%H%M%S%.6f").to_string();
            info!("qte name:{}, sim: {:.6}", png_name, max_val);
            let _ = tx.send(format!("qte name:{}, sim: {:.6}", png_name, max_val));
            // 确保目录存在 (Rust 不会自动创建目录，需使用 std::fs::create_dir_all)
            let file_path = format!("{}/{}.png", QTE_DIR, png_name);
            // 保存图片 (params 传空 Vector)
            info!("qte image name: {}, sim: {}.", file_path, max_val);
            let _ = tx.send(format!(
                "Save qte image, image path: {}, sim: {}",
                file_path, max_val
            ));
            if imwrite(&file_path, &resized_rappy_qte_shot, &Vector::new()).is_err() {
                error!("Failed to save QTE image to {}", file_path);
            }
            return true;
        }
        false
    }

    fn wait_for_key_ready(
        &self,
        capture: &DxgiCapture,
        bet_coin_is_one: &mut bool,
        tx: &Sender<String>,
    ) {
        info!("Waiting for key ready...");
        tx.send("Waiting for Key ready.".to_string()).unwrap();
        
        // 添加超时机制，最多等待60秒
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(60);
        
        // scroll灯亮起但游戏中开始(回车键)不可用，等待
        while !check_game_shot(
            capture,
            &CapturePos::key_ready(self.offset_x, self.offset_y),
            &TemplateImg::KEY_READY,
            0.9,
            true,
        ) && WindowsKeyboard::state()
        {
            if start_time.elapsed() > timeout {
                error!("Key ready detection timeout after 60 seconds");
                let _ = tx.send("Key ready detection timeout after 60 seconds".to_string());
                break;
            }
            sleep(Duration::from_millis(2000));
        }
        // 每次更新下状态
        *bet_coin_is_one = check_game_shot(
            capture,
            &CapturePos::coin_count(self.offset_x, self.offset_y),
            &TemplateImg::COIN_ONE,
            0.85,
            true,
        );
        info!("Key ready, [(bet coin nums == 1) : {}].", *bet_coin_is_one);
        tx.send(format!(
            "Key ready, [(bet coin nums == 1) : {}]",
            *bet_coin_is_one
        ))
        .unwrap();
    }

    fn try_increase_coin_while_energy_is_four(
        &self,
        capture: &DxgiCapture,
        keyboard: &WindowsKeyboard,
        bet_coin_is_one: &mut bool,
        burst: &mut bool,
        tx: &Sender<String>,
    ) {
        info!(
            "Trying to increase coin for the keyboard, bet coin is one: {}.",
            bet_coin_is_one
        );
        tx.send(format!(
            "Trying to increase coin for the keyboard, bet coin is one: {}.",
            bet_coin_is_one
        ))
        .unwrap();
        if *bet_coin_is_one
            && check_game_shot(
                capture,
                &CapturePos::energy_four(self.offset_x, self.offset_y),
                &TemplateImg::ENERGY_FOUR,
                0.85,
                true,
            )
        {
            info!("Bet coin = 1,  energy = 4, increase bet coin.");
            tx.send("Bet coin = 1,  energy = 4, increase bet coin.".to_string())
                .unwrap();
            // 没到5枚硬币时连续按上键(最大20次)
            for i in 0..=20 {
                if !check_game_shot(
                    &capture,
                    &CapturePos::coin_count(self.offset_x, self.offset_y),
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
            info!("Increase bet coin finished");
            tx.send("Increase bet coin finished".to_string()).unwrap();
            // 已经执行增加coin操作, bet_coin_one设置为false
            *bet_coin_is_one = false;
        } else {
            info!("No need to increase coin.");
            tx.send("No need to increase coin.".to_string()).unwrap();
        }
    }

    fn try_decrease_coin_while_energy_is_zero(
        &self,
        capture: &DxgiCapture,
        keyboard: &WindowsKeyboard,
        bet_coin_is_one: &mut bool,
        tx: &Sender<String>,
    ) {
        info!(
            "Trying to decrease coin for the keyboard, bet coin is one: {}.",
            bet_coin_is_one
        );
        tx.send(format!(
            "Trying to decrease coin for the keyboard, bet coin is one: {}.",
            bet_coin_is_one
        ))
        .unwrap();
        if !*bet_coin_is_one
            && check_game_shot(
                &capture,
                &CapturePos::energy_zero(self.offset_x, self.offset_y),
                &TemplateImg::ENERGY_ZERO,
                0.85,
                true,
            )
        {
            info!("Bet coin > 1,  energy = 0, decrease bet coin.");
            tx.send("Bet coin > 1,  energy = 0, decrease bet coin.".to_string())
                .unwrap();
            for _i in 0..=20 {
                if !check_game_shot(
                    &capture,
                    &CapturePos::coin_count(self.offset_x, self.offset_y),
                    &TemplateImg::COIN_ONE,
                    0.85,
                    true,
                ) {
                    keyboard.decrease_rappy_coin(1);
                }
            }
            *bet_coin_is_one = true;
        } else {
            info!("No need to decrease coin.");
            tx.send("No need to decrease coin.".to_string()).unwrap();
        }
    }

    fn process_rappy_qte(&self, capture: &DxgiCapture, burst: &mut bool, tx: &Sender<String>) {
        info!(
            "Check if  processing rappy qte needed, rappy burst status: {}.",
            *burst
        );
        tx.send(format!(
            "Check if  processing rappy qte needed, rappy burst status: {}.",
            *burst
        ))
        .unwrap();
        if check_game_shot(
            capture,
            &CapturePos::target(self.offset_x, self.offset_y),
            &TemplateImg::TARGET,
            0.7,
            true,
        ) || *burst
        {
            info!("Rappy target appear, wait for qte.");
            tx.send("Rappy target appear, wait for qte.".to_string())
                .unwrap();
            // 等qte完全开始
            sleep(Duration::from_millis(3000));
            
            // 添加超时机制，最多等待30秒
            let start_time = std::time::Instant::now();
            let timeout = Duration::from_secs(30);
            
            while !self.check_qte_appear(capture, tx) {
                if start_time.elapsed() > timeout {
                    error!("QTE detection timeout after 30 seconds");
                    let _ = tx.send("QTE detection timeout after 30 seconds".to_string());
                    *burst = false;
                    return;
                }
                sleep(Duration::from_millis(100));
            }
            info!("qte appear, ready.");
            tx.send("qte appear, ready.".to_string()).unwrap();
            // 出现QTE, 排除掉rappy burst的可能
            *burst = false;
        } else {
            info!("No rappy qte appear.");
            tx.send("No rappy qte appear.".to_string()).unwrap();
        }
    }
}

pub fn auto_rappy(ctx: &Context, tx: &Sender<String>) -> Result<String, Error> {
    let window_name = "PHANTASY STAR ONLINE 2";

    match search_window_by_title(window_name) {
        Some(hwnd) => {
            if let Some((offset_x, offset_y)) = get_window_client_offset(hwnd) {
                let auto_rappy = AutoRappy { offset_x, offset_y };
                let mut capture = DxgiCapture::new(hwnd)?;
                // 检查赌场币是否为1
                let mut bet_coin_is_one = check_game_shot(
                    &capture,
                    &CapturePos::coin_count(offset_x, offset_y),
                    &TemplateImg::COIN_ONE,
                    0.85,
                    true,
                );
                info!("Start task, check bet coin nums == 1: {}", bet_coin_is_one);
                tx.send(format!(
                    "Start task, check bet coin nums == 1: {}",
                    bet_coin_is_one
                ))
                .unwrap();
                // pse burst状态, 初始为false
                let mut burst = false;
                let mut keyboard = WindowsKeyboard::new(hwnd);
                loop {
                    if WindowsKeyboard::state() {
                        // 等待按下回车
                        auto_rappy.wait_for_key_ready(&capture, &mut bet_coin_is_one, tx);
                        // 赌场币为1枚,能量为4格时,增加赌场币到5枚,等待满能量pse
                        auto_rappy.try_increase_coin_while_energy_is_four(
                            &capture,
                            &keyboard,
                            &mut bet_coin_is_one,
                            &mut burst,
                            tx,
                        );
                        // 赌场币不为1枚，能力不足4格，将赌场币降低到1
                        auto_rappy.try_decrease_coin_while_energy_is_zero(
                            &capture,
                            &keyboard,
                            &mut bet_coin_is_one,
                            tx,
                        );
                        auto_rappy.process_rappy_qte(&capture, &mut burst, tx);
                        if check_game_shot(
                            &capture,
                            &CapturePos::key_ready(offset_x, offset_y),
                            &TemplateImg::KEY_READY,
                            0.9,
                            true,
                        ) {
                            info!("Press enter key.");
                            tx.send("Press enter key.".to_string()).unwrap();
                            keyboard.play_rappy();
                        }
                        if !check_game_shot(
                            &capture,
                            &CapturePos::coin_count(auto_rappy.offset_x, auto_rappy.offset_y),
                            &TemplateImg::COIN_ONE,
                            0.85,
                            true,
                        ) && !check_game_shot(
                            &capture,
                            &CapturePos::coin_count(auto_rappy.offset_x, auto_rappy.offset_y),
                            &TemplateImg::COIN_FIVE,
                            0.85,
                            true,
                        ) {
                            // 画面错位，刷新下这个窗口试下
                            info!("Invalid window handle, updating window...");
                            tx.send("Invalid window handle, updating window...".to_string())
                                .unwrap();
                            if let Some(_hwnd) = update_window(&window_name) {
                                capture.update_hwnd(_hwnd);
                                keyboard = WindowsKeyboard::new(_hwnd);
                            }
                        }
                    } else {
                        info!("Task ended.");
                        let _ = tx.send("Task ended.".to_string());
                        break;
                    }
                    ctx.request_repaint(); // 强制 UI 刷新以看到新日志
                }
            }
            Ok("Task ended.".to_string())
        }
        None => {
            error!("Search window: {} failed", window_name);
            Ok(format!("Search window: {} failed", window_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dxgi_capture::show_image;
    use crate::windows_utils::get_window_client_offset;

    #[test]
    fn test_grab_and_check() -> Result<(), Error> {
        match search_window_by_title("PHANTASY STAR ONLINE 2") {
            Some(hwnd) => {
                let capture = DxgiCapture::new(hwnd)?;
                info!("Check key ready");
                let template = TemplateImg::KEY_READY;
                let img = &template.img;
                info!(
                    "Template image channel: {}, rows: {}, cols: {}",
                    img.channels(),
                    img.rows(),
                    img.cols()
                );
                if let Some((offset_x, offset_y)) = get_window_client_offset(hwnd) {
                    let is_similar = check_game_shot(
                        &capture,
                        &CapturePos::key_ready(offset_x, offset_y),
                        &template,
                        0.85,
                        true,
                    );
                    info!("Similar key_ready_shot: {}", is_similar);
                }
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
                let capture = DxgiCapture::new(hwnd)?;
                if let Some((offset_x, offset_y)) = get_window_client_offset(hwnd) {
                    let coin_one = capture.grab(&CapturePos::coin_count(offset_x, offset_y));
                    show_image(&coin_one);
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    #[test]
    fn test_match_qte() -> Result<(), Error> {
        match search_window_by_title("PHANTASY STAR ONLINE 2") {
            Some(hwnd) => {
                let capture = DxgiCapture::new(hwnd)?;
                let (tx, _) = std::sync::mpsc::channel();
                if let Some((offset_x, offset_y)) = get_window_client_offset(hwnd) {
                    let auto_rappy = AutoRappy { offset_x, offset_y };
                    auto_rappy.check_qte_appear(&capture, &tx);
                }
                Ok(())
            }
            None => Ok(()),
        }
    }
}
