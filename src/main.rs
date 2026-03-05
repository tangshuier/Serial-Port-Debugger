#![windows_subsystem = "windows"]
use eframe::egui;
use std::io::{Read, Write};

// 全局变量用于在后台线程和主线程之间传递下载状态
static mut DOWNLOAD_PROGRESS: f32 = 0.0;
static mut DOWNLOAD_COMPLETED: bool = false;
static mut DOWNLOAD_RESULT: Option<Result<(), Box<dyn std::error::Error>>> = None;

// 导入拆分的模块
mod config;
mod serial;
mod utils;
mod ui;
mod cloud;
mod dataflow;
mod update;

// 重新导出必要的类型
pub use config::AppConfig;
pub use serial::SerialManager;
pub use utils::DisplayMode;
pub use cloud::CloudManager;
pub use dataflow::DataflowManager;

// 应用程序状态
struct SerialMonitor {
    // 串口管理器
    pub serial_manager: SerialManager,
    // 云端通信管理器
    pub cloud_manager: CloudManager,
    // 数据流转管理器
    pub dataflow_manager: DataflowManager,
    // 数据显示
    pub received_data: String,
    pub display_mode: DisplayMode,
    pub receive_encoding: String,
    // 滚动位置跟踪
    pub should_auto_scroll: bool,
    // 窗口位置和大小
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_width: f32,
    pub window_height: f32,
    // 窗口位置设置标志
    pub has_set_window_position: bool,
    // 发送数据
    pub send_data: String,
    pub send_encoding: String,
    pub send_newline: bool,
    // 主题设置
    pub is_dark_mode: bool,
    // 右侧设置面板
    pub show_settings_panel: bool,
    // 标签页状态
    pub current_tab: String,
    // 云端配置窗口
    pub show_cloud_config_window: bool,
    // 临时输入字段
    pub new_subscribe_topic: String,
    pub new_publish_topic: String,
    // 错误提示窗口
    pub show_error_window: bool,
    pub error_message: String,
    // 快捷指令设置
    pub shortcuts: Vec<String>,
    pub new_shortcut: String,
    pub show_shortcut_window: bool,
    pub editing_shortcut_index: Option<usize>,
    // 更新检测设置
    pub show_update_window: bool,
    pub update_available: bool,
    pub ignore_update: bool,
    pub check_for_updates: bool,
    // 下载进度
    pub show_download_window: bool,
    pub download_progress: f32,
    pub download_error: Option<String>,
    // 重启标志
    pub restart_needed: bool,
}

impl SerialMonitor {
    // 从配置创建实例
    fn from_config(config: &AppConfig) -> Self {
        Self {
            serial_manager: SerialManager::default(),
            cloud_manager: CloudManager::from_config(
                &config.cloud_service,
                &config.cloud_protocol,
                &config.cloud_uid,
                &config.cloud_subscribe_topics,
                &config.cloud_publish_topics,
                config.show_cloud_debug_info
            ),
            dataflow_manager: DataflowManager::from_config(
                config.dataflow_enabled,
                crate::dataflow::ConnectionMode::Direct
            ),
            received_data: String::new(),
            display_mode: match config.display_mode.as_str() {
                "Hex" => DisplayMode::Hex,
                "Binary" => DisplayMode::Binary,
                _ => DisplayMode::UTF8,
            },
            receive_encoding: config.receive_encoding.clone(),
            should_auto_scroll: config.should_auto_scroll,
            window_x: config.window_x,
            window_y: config.window_y,
            window_width: config.window_width,
            window_height: config.window_height,
            has_set_window_position: false,
            send_data: String::new(),
            send_encoding: config.send_encoding.clone(),
            send_newline: config.send_newline,
            is_dark_mode: config.is_dark_mode,
            show_settings_panel: config.show_settings_panel,
            current_tab: "串口设置".to_string(),
            show_cloud_config_window: false,
            new_subscribe_topic: String::new(),
            new_publish_topic: String::new(),
            show_error_window: false,
            error_message: String::new(),
            shortcuts: config.shortcuts.clone(),
            new_shortcut: String::new(),
            show_shortcut_window: false,
            editing_shortcut_index: None,
            // 更新检测设置
            show_update_window: false,
            update_available: false,
            ignore_update: false,
            check_for_updates: config.check_for_updates,
            // 下载进度
            show_download_window: false,
            download_progress: 0.0,
            download_error: None,
            // 重启标志
            restart_needed: false,
        }
    }
}

impl Default for SerialMonitor {
    fn default() -> Self {
        // 使用默认配置
        let config = AppConfig::default();
        Self::from_config(&config)
    }
}

impl SerialMonitor {
    // 保存配置
    fn save_config(&self) {
        // 打印保存的配置信息
        println!("Saving config with window_x: {:?}, window_y: {:?}, window_width: {}, window_height: {}, is_dark_mode: {}", 
                 self.window_x, self.window_y, self.window_width, self.window_height, self.is_dark_mode);
        
        let config = AppConfig {
            selected_port: self.serial_manager.selected_port.as_ref().map(|port| port.split('\t').next().unwrap().to_string()),
            baud_rate: self.serial_manager.baud_rate,
            data_bits: match self.serial_manager.data_bits {
                serialport::DataBits::Five => 5,
                serialport::DataBits::Six => 6,
                serialport::DataBits::Seven => 7,
                serialport::DataBits::Eight => 8,
            },
            parity: match self.serial_manager.parity {
                serialport::Parity::None => 0,
                serialport::Parity::Odd => 1,
                serialport::Parity::Even => 2,
            },
            stop_bits: match self.serial_manager.stop_bits {
                serialport::StopBits::One => 1,
                serialport::StopBits::Two => 2,
            },
            display_mode: match self.display_mode {
                DisplayMode::UTF8 => "UTF8",
                DisplayMode::Hex => "Hex",
                DisplayMode::Binary => "Binary",
            }.to_string(),
            receive_encoding: self.receive_encoding.clone(),
            should_auto_scroll: self.should_auto_scroll,
            send_encoding: self.send_encoding.clone(),
            send_newline: self.send_newline,
            is_dark_mode: self.is_dark_mode,
            show_settings_panel: self.show_settings_panel,
            use_cloud_communication: self.current_tab == "云端通信",
            cloud_service: self.cloud_manager.service.clone(),
            cloud_protocol: self.cloud_manager.protocol.clone(),
            cloud_uid: self.cloud_manager.uid.clone(),
            cloud_subscribe_topics: self.cloud_manager.subscribe_topics.clone(),
            cloud_publish_topics: self.cloud_manager.publish_topics.clone(),
            show_cloud_debug_info: self.cloud_manager.show_debug_info,
            dataflow_enabled: self.dataflow_manager.enabled,
            use_dedicated_firmware: matches!(self.dataflow_manager.connection_mode, crate::dataflow::ConnectionMode::Firmware),
            shortcuts: self.shortcuts.clone(),
            window_x: self.window_x,
            window_y: self.window_y,
            window_width: self.window_width,
            window_height: self.window_height,
            check_for_updates: self.check_for_updates,
            last_update_check: Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
        };
        
        config.save();
    }

    // 处理接收到的数据
    fn process_received_data(&mut self) {
        while let Some(bytes) = self.serial_manager.process_received_data() {
            // 根据显示模式格式化数据
            match self.display_mode {
                DisplayMode::UTF8 => {
                    // 使用 UTF-8 解析
                    let text = utils::try_decode(&bytes, &self.receive_encoding);
                    self.received_data.push_str(&text);
                    
                    // 如果启用了数据流转且连接到云端，将数据上传到云端
                    if let Err(e) = self.dataflow_manager.process_serial_to_cloud(&text, &mut self.cloud_manager) {
                        if self.cloud_manager.show_debug_info {
                            self.received_data.push_str(&format!("数据流转上传失败: {}\n", e));
                        }
                    }
                }
                DisplayMode::Hex => {
                    let hex_str = bytes.iter().map(|b| format!("{:02X} ", b)).collect::<String>();
                    self.received_data.push_str(&hex_str);
                    self.received_data.push('\n');
                }
                DisplayMode::Binary => {
                    let bin_str = bytes.iter().map(|b| format!("{:08b} ", b)).collect::<String>();
                    self.received_data.push_str(&bin_str);
                    self.received_data.push('\n');
                }
            }
        }
        
        // 处理云端接收到的数据
        self.process_cloud_received_data();
    }
    
    // 处理云端接收到的数据
    fn process_cloud_received_data(&mut self) {
        while let Some(response) = self.cloud_manager.process_received_data() {
            if self.cloud_manager.show_debug_info {
                self.received_data.push_str(&format!("云端接收: {}", response));
            }
            
            // 如果启用了数据流转，将云端数据通过串口下发
            match self.dataflow_manager.process_cloud_to_serial(&response, &self.serial_manager.port, &self.cloud_manager) {
                Err(e) => {
                    if self.cloud_manager.show_debug_info {
                        self.received_data.push_str(&format!("数据流转下发失败: {}\n", e));
                    }
                }
                Ok(Some(actual_sent)) => {
                    if self.cloud_manager.show_debug_info {
                        // 检查串口是否连接
                        if self.serial_manager.port.is_none() {
                            // 串口未连接，显示提示信息
                            self.received_data.push_str(&format!("串口未连接，流转数据为：{}\n", actual_sent));
                        } else {
                            // 串口已连接，显示正常的流转信息
                            self.received_data.push_str(&format!("数据流转下发: {}\n", actual_sent));
                        }
                    }
                }
                Ok(None) => {
                    // 没有发送数据，不显示
                }
            }
        }
    }
}

impl eframe::App for SerialMonitor {
    // 程序退出时保存配置
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // 断开云端连接
        self.cloud_manager.disconnect();
        self.save_config();
    }
    
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 持续请求重绘，确保即使没有用户交互也能实时更新
        ctx.request_repaint();
        
        // 设置主题
        ctx.set_visuals(if self.is_dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        
        // 每500毫秒扫描一次串口，自动检测CH340设备
        static mut LAST_SCAN_TIME: f64 = 0.0;
        let current_time = ctx.input(|i| i.time);
        unsafe {
            if current_time - LAST_SCAN_TIME >= 0.5 {
                self.serial_manager.scan_ports();
                LAST_SCAN_TIME = current_time;
            }
        }
        
        // 每60秒发送一次心跳，保持云端连接
        static mut LAST_HEARTBEAT_TIME: f64 = 0.0;
        unsafe {
            if current_time - LAST_HEARTBEAT_TIME >= 60.0 {
                if self.cloud_manager.connected {
                    if let Err(e) = self.cloud_manager.send_heartbeat() {
                        if self.cloud_manager.show_debug_info {
                            self.received_data.push_str(&format!("心跳发送失败: {}\n", e));
                        }
                    }
                }
                LAST_HEARTBEAT_TIME = current_time;
            }
        }
        
        // 启动时检查更新
        static mut CHECKED_UPDATE: bool = false;
        unsafe {
            if !CHECKED_UPDATE && self.check_for_updates {
                CHECKED_UPDATE = true;
                // 直接检查更新（在主线程中）
                match update::check_for_updates() {
                    Ok(available) => {
                        if available {
                            // 检测到更新，显示更新窗口
                            self.show_update_window = true;
                            self.update_available = true;
                        }
                    },
                    Err(e) => {
                        println!("检查更新失败: {:?}", e);
                    }
                }
            }
        }
        
        // 处理接收到的数据
        self.process_received_data();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // 渲染UI
            ui::render_ui(ui, self);
        });
        
        // 渲染云端配置窗口
        ui::render_cloud_config_window(ctx, self);
        
        // 渲染错误提示窗口
        ui::render_error_window(ctx, self);
        
        // 渲染快捷指令编辑窗口
        ui::render_shortcut_window(ctx, self);
        
        // 渲染更新提示窗口
        if self.show_update_window {
            egui::Window::new("软件更新")
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("检测到新版本，是否更新？");
                    
                    ui.separator();
                    
                    let mut ignore_update = self.ignore_update;
                    ui.checkbox(&mut ignore_update, "不再提醒");
                    self.ignore_update = ignore_update;
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.button("更新").clicked() {
                            // 显示下载进度窗口
                            self.show_download_window = true;
                            self.download_progress = 0.0;
                            self.download_error = None;
                            self.show_update_window = false;
                            
                            // 重置全局下载状态
                            unsafe {
                                DOWNLOAD_PROGRESS = 0.0;
                                DOWNLOAD_COMPLETED = false;
                                DOWNLOAD_RESULT = None;
                            }
                            
                            // 在后台线程中下载更新
                            let ctx_for_closure = ctx.clone();
                            let ctx_for_completion = ctx.clone();
                            std::thread::spawn(move || {
                                let result = update::download_update_with_progress(move |progress| {
                                    // 在主线程中更新进度
                                    ctx_for_closure.request_repaint();
                                    // 这里需要使用 Arc<Mutex> 来安全地更新进度
                                    // 为了简化，我们先使用一个全局变量
                                    // 注意：这不是线程安全的，实际应用中应该使用 Arc<Mutex>
                                    unsafe {
                                        DOWNLOAD_PROGRESS = progress;
                                    }
                                });
                                
                                // 下载完成后更新状态
                                ctx_for_completion.request_repaint();
                                unsafe {
                                    DOWNLOAD_COMPLETED = true;
                                    DOWNLOAD_RESULT = Some(result);
                                }
                            });
                        }
                        
                        if ui.button("忽略").clicked() {
                            // 如果用户选择不再提醒，禁用更新检测
                            if self.ignore_update {
                                self.check_for_updates = false;
                                self.save_config();
                            }
                            self.show_update_window = false;
                        }
                    });
                });
        }
        
        // 渲染下载进度窗口
        if self.show_download_window {
            // 更新进度
            unsafe {
                self.download_progress = DOWNLOAD_PROGRESS;
                if DOWNLOAD_COMPLETED {
                    // 使用 &raw const 来创建原始指针并读取静态变量
                    let result = std::ptr::read(&raw const DOWNLOAD_RESULT);
                    // 重置下载状态
                    DOWNLOAD_COMPLETED = false;
                    DOWNLOAD_RESULT = None;
                    
                    match result {
                        Some(Ok(_)) => {
                            // 更新成功，提示用户重启软件
                            self.error_message = "更新成功，是否立即重启软件？".to_string();
                            self.show_error_window = true;
                            self.show_download_window = false;
                        }
                        Some(Err(e)) => {
                            // 更新失败，显示错误信息
                            self.error_message = format!("更新失败: {:?}", e);
                            self.show_error_window = true;
                            self.show_download_window = false;
                        }
                        None => {}
                    }
                }
            }
            
            egui::Window::new("下载更新")
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("正在下载更新，请稍候...");
                    ui.add_space(10.0);
                    ui.add(egui::ProgressBar::new(self.download_progress).text(format!("{:.1}%", self.download_progress * 100.0)));
                    ui.add_space(10.0);
                    ui.label("请勿关闭此窗口...");
                });
        }
        
        // 检查是否需要重启
        if self.restart_needed {
            // 获取当前可执行文件路径
            if let Ok(exe_path) = std::env::current_exe() {
                // 创建批处理文件来删除旧版本并启动新版本
                let batch_path = exe_path.with_extension("bat");
                let old_exe_path = exe_path.with_extension("old.exe");
                
                // 构建批处理文件内容（使用ASCII字符，避免编码问题）
                let batch_content = format!(r#"
@echo off
:: Ensure we're in the correct directory
cd /d "{}"
:: Output current directory
echo Current directory: %CD%
:: Wait 5 seconds for main program to exit
echo Waiting 5 seconds...
timeout /t 5 /nobreak > nul
:: Delete old version files with timestamp
echo Deleting old version files...
del "串口调试器.old_*.exe" /f /q
if errorlevel 1 (
    echo Delete failed, error code: %errorlevel%
) else (
    echo Delete successful
)
:: Wait 2 seconds to ensure deletion is complete
timeout /t 2 /nobreak > nul
:: Delete batch file itself
echo Deleting batch file...
del "%~f0" /f /q
"#, exe_path.parent().unwrap().display());
                
                // Write batch file with ASCII encoding to avoid garbled characters
                if let Ok(mut file) = std::fs::File::create(&batch_path) {
                    // Convert to ASCII bytes, replacing non-ASCII characters with '?'
                    let ascii_bytes: Vec<u8> = batch_content.chars()
                        .map(|c| if c.is_ascii() { c as u8 } else { b'?' })
                        .collect();
                    
                    if let Ok(_) = file.write_all(&ascii_bytes) {
                        // Start batch file with /c to close window after execution
                        let _ = std::process::Command::new("cmd.exe")
                            .arg("/c")
                            .arg(batch_path)
                            .spawn();
                    }
                }
                
                // 启动新的实例
                let _ = std::process::Command::new(exe_path)
                    .spawn();
            }
            // 退出当前应用
            std::process::exit(0);
        }
    }
}

fn main() {
    // 配置字体
    let mut fonts = egui::FontDefinitions::default();
    
    // 添加中文字体
    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("C:/Windows/Fonts/simhei.ttf")),
    );
    
    // 将中文字体添加到默认字体家族
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "my_font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "my_font".to_owned());
    
    // 加载图标
    let icon_bytes = include_bytes!("串口设置.png");
    let icon = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    let _icon_data = egui::IconData {
        rgba: icon.into_raw(),
        width,
        height,
    };
    
    // 尝试加载配置以获取窗口大小和位置
    let config = AppConfig::load();
    println!("Main: Loaded config with window_width: {}, window_height: {}, window_x: {:?}, window_y: {:?}", 
             config.window_width, config.window_height, config.window_x, config.window_y);
    
    // 构建视口配置
    let mut viewport_builder = egui::ViewportBuilder::default();
    
    // 设置最小窗口尺寸
    viewport_builder = viewport_builder.with_min_inner_size([700.0, 500.0]);
    
    // 设置窗口大小
    if config.window_width > 0.0 && config.window_height > 0.0 {
        println!("Main: Setting window size to {}x{}", config.window_width, config.window_height);
        viewport_builder = viewport_builder.with_inner_size([config.window_width, config.window_height]);
    } else {
        println!("Main: Using default window size 800x600");
        viewport_builder = viewport_builder.with_inner_size([800.0, 600.0]);
    }
    
    // 设置窗口位置
    if let (Some(x), Some(y)) = (config.window_x, config.window_y) {
        println!("Main: Setting window position to ({}, {})\n", x, y);
        viewport_builder = viewport_builder.with_position([x, y]);
    }
    
    // 构建应用程序配置
    let native_options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };
    
    // 运行应用程序
    let config_clone = config.clone();
    eframe::run_native(
        "串口调试助手 v1.2.3",
        native_options,
        Box::new(move |cc| {
            // 设置字体
            cc.egui_ctx.set_fonts(fonts);
            // 使用加载的配置创建实例
            Box::new(SerialMonitor::from_config(&config_clone))
        }),
    ).expect("Failed to run application");
}
