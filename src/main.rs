#![cfg_attr(windows, windows_subsystem = "windows")]

use crate::auto_rappy::{check_or_create_dir, QTE_DIR, TARGET_DIR};
use crate::keyboard_utils::WindowsKeyboard;
use crate::logging::init_logger;
use eframe::egui;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use windows::core::Error;

mod auto_rappy;
mod capture_settings;
mod dxgi_capture;
mod keyboard_utils;
mod logging;
mod rappy_checker;
mod template_img;
mod windows_utils;

pub struct RappyApp {
    is_running: bool,
    logs: String,
    // 用于接收从工作线程传回的日志
    rx: Receiver<String>,
    // 用于向工作线程发送停止信号（简单起见这里用布尔变量控制）
    tx: Sender<String>,
}

impl RappyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            is_running: false,
            logs: String::from("Program Ready...\n"),
            rx,
            tx,
        }
    }
}

impl eframe::App for RappyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. 处理接收到的日志
        while let Ok(msg) = self.rx.try_recv() {
            let date_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            self.logs.push_str(&format!("{}: {}\n", date_time.to_string(), msg));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Happy Rappy Machine");

            // 2. 开始/停止 按钮
            let (button_label, button_color) = if self.is_running {
                ("⏸ Stop Task", egui::Color32::from_rgb(200, 60, 60)) // 红色
            } else {
                ("▶ Start Task", egui::Color32::from_rgb(60, 160, 60)) // 绿色
            };
            // 2. 使用 ui.add 构建自定义按钮
            let button_design = egui::Button::new(
                egui::RichText::new(button_label)
                    .color(egui::Color32::WHITE) // 文字设为白色
                    .strong()                    // 文字加粗
                    .size(30.0)
            ).fill(button_color)                // 设置按钮背景色
                .corner_radius(10)
                .min_size(egui::vec2(240.0,70.0));

            if ui.add(button_design).clicked() {
                self.is_running = !self.is_running;

                if self.is_running {
                    WindowsKeyboard::start_app();
                    let tx = self.tx.clone();
                    let ctx_clone = ctx.clone(); // 用于在子线程触发 UI 刷新
                    // 模拟后台任务
                    thread::spawn(move || {
                        let _ = auto_rappy::auto_rappy(&ctx_clone, &tx);
                    });
                } else {
                    WindowsKeyboard::stop_app();
                }
            }

            ui.separator();

            // 3. 类似控制台的文本框
            ui.label("Output log:");
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true) // 自动滚动到底部
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.logs)
                            .font(egui::TextStyle::Monospace) // 使用等宽字体像控制台
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .lock_focus(true),
                    );
                });
        });
    }
}

fn load_icon(bytes: &[u8]) -> egui::IconData {
    let image = image::load_from_memory(bytes)
        .expect("无法加载图标：请确保图片格式正确（如 .png）")
        .into_rgba8();

    let (width, height) = image.dimensions();

    egui::IconData {
        rgba: image.into_raw(), // 提取原始 RGBA 像素数据
        width,
        height,
    }
}

fn main() -> Result<(), Error> {
    let _logger = init_logger("info");
    let icon_bytes = include_bytes!("../resources/ico/rappy.ico");
    let icon = load_icon(icon_bytes);
    check_or_create_dir(TARGET_DIR);
    check_or_create_dir(QTE_DIR);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 320.0])
            .with_icon(icon),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Rappy Machine UI",
        options,
        Box::new(|cc| Ok(Box::new(RappyApp::new(cc)))),
    );
    Ok(())
}
