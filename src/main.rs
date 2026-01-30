#![cfg_attr(windows, windows_subsystem = "windows")]

use crate::auto_rappy::{QTE_DIR, TARGET_DIR, check_or_create_dir};
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
            // 1. 檢查目前行數
            let line_count = self.logs.lines().count();

            // 2. 如果已經達到或超過 20 條，移除第一行
            if line_count >= 20 {
                if let Some(first_newline_pos) = self.logs.find('\n') {
                    // 移除從開頭到第一個換行符號（含）的內容
                    self.logs.replace_range(..first_newline_pos + 1, "");
                }
            }
            self.logs
                .push_str(&format!("{}: {}\n", date_time.to_string(), msg));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.style_mut(|s| s.interaction.tooltip_delay = 0.1);
            let label_text = egui::RichText::new("❓ Usage Instructions").underline();
            let label = ui.add(egui::Label::new(label_text).sense(egui::Sense::click()));
            // 使用 on_hover_ui 來實現標準格式
            label.on_hover_ui(|ui| {
                ui.set_max_width(300.0); // 限制寬度，避免長文本變成一橫條

                ui.vertical(|ui| {
                    ui.heading("Configuration Steps");
                    ui.add_space(4.0);

                    // 第 1 點
                    ui.strong("1. System Requirements");
                    ui.label("• Resolution: 3840x2160");
                    ui.label("• DPI Scaling: 150% (Important)");
                    ui.separator();

                    // 第 2 點
                    ui.strong("2. In-Game Settings");
                    ui.label("• Mode: 1600x900 Windowed");
                    ui.horizontal(|ui| {
                        ui.label("• Path:");
                        ui.code("Options > Controls > Guide");
                    });
                    ui.label("• Set to: 'Keyboard Type'");
                    ui.separator();

                    // 第 3 & 4 點
                    ui.strong("3. Execution");
                    ui.label("Enter Rappy Machine -> Click 'Start Task'");

                    ui.add_space(4.0);
                    ui.colored_label(egui::Color32::GRAY, "Click label for full details");
                });
            });

            ui.separator();
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
                    .strong() // 文字加粗
                    .size(30.0),
            )
            .fill(button_color) // 设置按钮背景色
            .corner_radius(10)
            .min_size(egui::vec2(240.0, 70.0));

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
        "Happy Rappy Machine",
        options,
        Box::new(|cc| Ok(Box::new(RappyApp::new(cc)))),
    );
    Ok(())
}
