#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui;
use std::io::Write;

// 全局原子变量用于在后台线程和主线程之间传递状态
use std::sync::atomic::AtomicBool;
static VERSIONS_LOADED: AtomicBool = AtomicBool::new(false);
static UPDATE_AVAILABLE: AtomicBool = AtomicBool::new(false);

// 全局变量用于存储版本列表和更新信息
use std::sync::{Arc, Mutex};

// 定义下载状态结构体
struct DownloadState {
    progress: f32,
    completed: bool,
    success: bool,
    error: Option<String>,
}

impl DownloadState {
    fn new() -> Self {
        Self {
            progress: 0.0,
            completed: false,
            success: false,
            error: None,
        }
    }
}

lazy_static::lazy_static! {
    pub static ref VERSIONS: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    pub static ref LATEST_VERSION: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    static ref DOWNLOAD_STATE: Arc<Mutex<DownloadState>> = Arc::new(Mutex::new(DownloadState::new()));
}


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
    pub show_timestamp: bool,
    // 编码缓存，用于处理跨数据包的编码单元
    pub encoding_cache: utils::EncodingCache,
    // 滚动位置跟踪
    pub should_auto_scroll: bool,
    // 窗口位置和大小
    pub window_x: Option<f32>,
    pub window_y: Option<f32>,
    pub window_width: f32,
    pub window_height: f32,
    // 发送数据
    pub send_data: String,
    pub send_encoding: String,
    pub send_newline: bool,
    pub send_hex: bool,
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
    pub shortcuts: Vec<(String, bool)>, // (指令内容, 是否包含换行符)
    pub new_shortcut: String,
    pub new_shortcut_newline: bool, // 新指令是否包含换行符
    pub show_shortcut_window: bool,
    pub editing_shortcut_index: Option<usize>,


    // 下载进度
    pub show_download_window: bool,
    pub download_progress: f32,
    pub download_error: Option<String>,
    // 重启标志
    pub restart_needed: bool,
    // 版本列表窗口
    pub show_versions_window: bool,
    pub versions: Vec<(String, String)>, // (版本号, 描述)
    pub is_loading_versions: bool,
    // 更新检查
    pub update_available: bool,
    pub latest_version: String,
    pub show_update_info_window: bool,
    // 定时器状态
    pub last_scan_time: f64,
    pub last_heartbeat_time: f64,
    pub has_checked_update: bool,
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
            show_timestamp: config.show_timestamp,
            encoding_cache: utils::EncodingCache::default(),
            should_auto_scroll: config.should_auto_scroll,
            window_x: config.window_x,
            window_y: config.window_y,
            window_width: config.window_width,
            window_height: config.window_height,
            send_data: String::new(),
            send_encoding: config.send_encoding.clone(),
            send_newline: config.send_newline,
            send_hex: config.send_hex,
            is_dark_mode: config.is_dark_mode,
            show_settings_panel: config.show_settings_panel,
            current_tab: "串口设置".to_string(),
            show_cloud_config_window: false,
            new_subscribe_topic: String::new(),
            new_publish_topic: String::new(),
            show_error_window: false,
            error_message: String::new(),
            shortcuts: config.shortcuts.iter().filter(|s| !s.is_empty()).map(|s| (s.clone(), true)).collect(), // 默认包含换行符，过滤空指令
            new_shortcut: String::new(),
            new_shortcut_newline: true, // 默认包含换行符
            show_shortcut_window: false,
            editing_shortcut_index: None,
            // 下载进度
            show_download_window: false,
            download_progress: 0.0,
            download_error: None,
            // 重启标志
            restart_needed: false,
            // 版本列表窗口
            show_versions_window: false,
            versions: Vec::new(),
            is_loading_versions: false,
            // 更新检查
            update_available: false,
            latest_version: "".to_string(),
            show_update_info_window: false,
            // 定时器状态
            last_scan_time: 0.0,
            last_heartbeat_time: 0.0,
            has_checked_update: false,
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
            show_timestamp: self.show_timestamp,
            send_encoding: self.send_encoding.clone(),
            send_newline: self.send_newline,
            send_hex: self.send_hex,
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
            shortcuts: self.shortcuts.iter().map(|(s, _)| s.clone()).collect(),
            window_x: self.window_x,
            window_y: self.window_y,
            window_width: self.window_width,
            window_height: self.window_height,

        };
        
        config.save();
    }

    // 处理接收到的数据
    fn process_received_data(&mut self) {
        while let Some(bytes) = self.serial_manager.process_received_data() {
            // 获取当前时间
            let timestamp = if self.show_timestamp {
                let now = chrono::Local::now();
                format!("[{}] ", now.format("%Y-%m-%d %H:%M:%S"))
            } else {
                String::new()
            };
            
            // 根据显示模式格式化数据
            match self.display_mode {
                DisplayMode::UTF8 => {
                    // 使用编码缓存处理跨数据包的编码单元
                    let processable_data = self.encoding_cache.process_data(&bytes);
                    if !processable_data.is_empty() {
                        // 使用 UTF-8 解析
                        let text = utils::try_decode(&processable_data, &self.receive_encoding);
                        self.received_data.push_str(&timestamp);
                        self.received_data.push_str(&text);
                        
                        // 如果启用了数据流转且连接到云端，将数据上传到云端
                        if let Err(e) = self.dataflow_manager.process_serial_to_cloud(&text, &mut self.cloud_manager) {
                            if self.cloud_manager.show_debug_info {
                                self.received_data.push_str(&format!("数据流转上传失败: {}\n", e));
                            }
                        }
                    }
                }
                DisplayMode::Hex => {
                    let hex_str = bytes.iter().map(|b| format!("{:02X} ", b)).collect::<String>();
                    self.received_data.push_str(&timestamp);
                    self.received_data.push_str(&hex_str);
                    self.received_data.push('\n');
                }
                DisplayMode::Binary => {
                    let bin_str = bytes.iter().map(|b| format!("{:08b} ", b)).collect::<String>();
                    self.received_data.push_str(&timestamp);
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
        
        let current_time = ctx.input(|i| i.time);
        
        // 程序启动时检查一次更新
        if !self.has_checked_update && current_time >= 1.0 { // 延迟1秒检查，确保程序完全启动
            self.has_checked_update = true;
            
            // 在后台线程中检查更新
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || {
                match crate::update::check_for_updates() {
                    Ok((update_available, latest_version)) => {
                        // 更新全局状态
                        UPDATE_AVAILABLE.store(update_available, std::sync::atomic::Ordering::Relaxed);
                        *crate::LATEST_VERSION.lock().unwrap() = latest_version;
                        // 通知主线程
                        ctx_clone.request_repaint();
                    },
                    Err(e) => {
                        println!("检查更新失败: {:?}", e);
                    }
                }
            });
        }
        
        // 每500毫秒扫描一次串口，自动检测CH340设备
        if current_time - self.last_scan_time >= 0.5 {
            self.serial_manager.scan_ports();
            self.last_scan_time = current_time;
        }
        
        // 每60秒发送一次心跳，保持云端连接
        if current_time - self.last_heartbeat_time >= 60.0 {
            if self.cloud_manager.connected {
                if let Err(e) = self.cloud_manager.send_heartbeat() {
                    if self.cloud_manager.show_debug_info {
                        self.received_data.push_str(&format!("心跳发送失败: {}\n", e));
                    }
                }
            }
            self.last_heartbeat_time = current_time;
        }
        
        // 检查版本列表是否加载完成
        if VERSIONS_LOADED.load(std::sync::atomic::Ordering::Relaxed) {
            VERSIONS_LOADED.store(false, std::sync::atomic::Ordering::Relaxed);
            // 获取版本列表
            self.versions = VERSIONS.lock().unwrap().clone();
            // 重置加载状态
            self.is_loading_versions = false;
        }
        
        // 检查更新状态
        if UPDATE_AVAILABLE.load(std::sync::atomic::Ordering::Relaxed) {
            self.update_available = true;
            self.latest_version = LATEST_VERSION.lock().unwrap().clone();
        } else {
            // 即使没有更新，也获取最新版本信息
            let latest_version = LATEST_VERSION.lock().unwrap().clone();
            if !latest_version.is_empty() {
                self.latest_version = latest_version;
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
        
        // 渲染更新信息窗口
        if self.show_update_info_window {
            egui::Window::new("更新信息")
                .resizable(false)
                .default_size([300.0, 200.0])
                .show(ctx, |ui| {
                    ui.heading("版本信息");
                    
                    // 显示当前版本
                    let current_version = update::get_local_version();
                    ui.label(format!("当前版本: {}", current_version));
                    
                    // 显示最新版本
                    let latest_version = self.latest_version.clone();
                    if self.update_available {
                        ui.label(egui::RichText::new(format!("最新版本: {}", latest_version)).color(egui::Color32::GREEN));
                        ui.label("有新版本可用！");
                    } else {
                        ui.label(format!("最新版本: {}", latest_version));
                        ui.label("当前已是最新版本");
                    }
                    
                    ui.add_space(20.0);
                    
                    // 按钮区域
                    ui.horizontal(|ui| {
                        // 查看更多版本按钮
                        if ui.button("查看更多版本").clicked() {
                            // 显示版本列表窗口
                            self.show_versions_window = true;
                            self.is_loading_versions = true;
                            
                            // 在后台线程中获取所有版本
                            let ctx_clone = ctx.clone();
                            std::thread::spawn(move || {
                                match crate::update::get_all_versions() {
                                    Ok(versions) => {
                                        // 更新版本列表
                                        *crate::VERSIONS.lock().unwrap() = versions;
                                        // 通知主线程
                                        ctx_clone.request_repaint();
                                        crate::VERSIONS_LOADED.store(true, std::sync::atomic::Ordering::Relaxed);
                                    },
                                    Err(e) => {
                                        println!("获取版本列表失败: {:?}", e);
                                    }
                                }
                            });
                            self.show_update_info_window = false;
                        }
                        
                        // 关闭按钮
                        if ui.button("关闭").clicked() {
                            self.show_update_info_window = false;
                        }
                    });
                });
        }
        
        // 渲染版本列表窗口
        if self.show_versions_window {
            egui::Window::new("所有版本")
                .resizable(false)
                .default_size([300.0, 200.0])
                .show(ctx, |ui| {
                    ui.heading("可用版本列表");
                    
                    if self.is_loading_versions {
                        ui.label("正在加载版本列表...");
                    } else if self.versions.is_empty() {
                        ui.label("未找到版本列表");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(120.0) // 限制滚动区域高度，大约显示5个版本
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                for (version, description) in &self.versions {
                                    let button = ui.button(version);
                                    if button.clicked() {
                                        // 显示下载进度窗口
                                        self.show_download_window = true;
                                        self.download_progress = 0.0;
                                        self.download_error = None;
                                        self.show_versions_window = false;
                                        
                                        // 重置全局下载状态
                                        {
                                            let mut state = DOWNLOAD_STATE.lock().unwrap();
                                            state.progress = 0.0;
                                            state.completed = false;
                                            state.success = false;
                                            state.error = None;
                                        }
                                        
                                        // 在后台线程中下载指定版本
                                        let ctx_for_closure = ctx.clone();
                                        let ctx_for_completion = ctx.clone();
                                        let version_clone = version.clone();
                                        std::thread::spawn(move || {
                                            let result = update::download_specific_version_with_progress(&version_clone, move |progress| {
                                                // 在主线程中更新进度
                                                ctx_for_closure.request_repaint();
                                                let mut state = DOWNLOAD_STATE.lock().unwrap();
                                                state.progress = progress;
                                            });
                                            
                                            // 下载完成后更新状态
                                            ctx_for_completion.request_repaint();
                                            let mut state = DOWNLOAD_STATE.lock().unwrap();
                                            state.completed = true;
                                            match result {
                                                Ok(_) => {
                                                    state.success = true;
                                                    state.error = None;
                                                }
                                                Err(e) => {
                                                    state.success = false;
                                                    state.error = Some(format!("{:?}", e));
                                                }
                                            }
                                        });
                                    }
                                    // 添加悬停提示
                                    if !description.is_empty() {
                                        button.on_hover_text(description);
                                    }
                                }
                            });
                    }
                    
                    ui.add_space(10.0);
                    if ui.button("关闭").clicked() {
                        self.show_versions_window = false;
                    }
                });
        }
        

        
        // 渲染下载进度窗口
        if self.show_download_window {
            // 更新进度
            {
                let mut state = DOWNLOAD_STATE.lock().unwrap();
                self.download_progress = state.progress;
                if state.completed {
                    // 安全地获取下载结果
                    let success = state.success;
                    let error = state.error.take();
                    // 重置下载状态
                    state.completed = false;
                    
                    if success {
                        // 更新成功，提示用户重启软件
                        self.error_message = "更新成功，是否立即重启软件？".to_string();
                        self.show_error_window = true;
                        self.show_download_window = false;
                    } else if let Some(err) = error {
                        // 更新失败，显示错误信息
                        self.error_message = format!("更新失败: {}", err);
                        self.show_error_window = true;
                        self.show_download_window = false;
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
                let _old_exe_path = exe_path.with_extension("old.exe");
                
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
        &format!("串口调试助手 v{}", env!("CARGO_PKG_VERSION")),
        native_options,
        Box::new(move |cc| {
            // 设置字体
            cc.egui_ctx.set_fonts(fonts);
            // 使用加载的配置创建实例
            let app = SerialMonitor::from_config(&config_clone);
            Box::new(app)
        }),
    ).expect("Failed to run application");
}
