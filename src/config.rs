use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

// 配置结构体
#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    // 串口配置
    pub selected_port: Option<String>,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: u8,
    pub stop_bits: u8,
    // 显示配置
    pub display_mode: String,
    pub receive_encoding: String,
    pub should_auto_scroll: bool,
    pub send_encoding: String,
    pub send_newline: bool,
    // 主题配置
    pub is_dark_mode: bool,
    // 右侧设置面板
    pub show_settings_panel: bool,
    // 云端通信设置
    pub use_cloud_communication: bool,
    pub cloud_service: String,
    pub cloud_protocol: String,
    pub cloud_uid: String,
    pub cloud_subscribe_topics: Vec<String>,
    pub cloud_publish_topics: Vec<String>,
    pub show_cloud_debug_info: bool,
    // 数据流转设置
            pub dataflow_enabled: bool,
            // 专用固件设置
            pub use_dedicated_firmware: bool,
            // 快捷指令设置
            pub shortcuts: Vec<String>,
            // 窗口配置
            pub window_x: Option<f32>,
            pub window_y: Option<f32>,
            pub window_width: f32,
            pub window_height: f32,

        }

        // 为 AppConfig 实现 Default trait
        impl Default for AppConfig {
            fn default() -> Self {
                Self {
                    selected_port: None,
                    baud_rate: 115200,
                    data_bits: 8,
                    parity: 0,
                    stop_bits: 1,
                    display_mode: "UTF8".to_string(),
                    receive_encoding: "UTF-8".to_string(),
                    should_auto_scroll: true,
                    send_encoding: "UTF-8".to_string(),
                    send_newline: false,
                    is_dark_mode: false,
                    show_settings_panel: false,
                    use_cloud_communication: false,
                    cloud_service: "巴法云".to_string(),
                    cloud_protocol: "TCP".to_string(),
                    cloud_uid: "".to_string(),
                    cloud_subscribe_topics: Vec::new(),
                    cloud_publish_topics: Vec::new(),
                    show_cloud_debug_info: true,
                    dataflow_enabled: true,
                    use_dedicated_firmware: false,
                    shortcuts: Vec::new(),
                    window_x: None,
                    window_y: None,
                    window_width: 800.0,
                    window_height: 600.0,
                }
            }
        }

impl AppConfig {
    // 加载配置
    pub fn load() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("com", "serialmonitor", "serial_monitor") {
            let config_path = proj_dirs.config_dir().join("config.toml");
            println!("Loading config from: {:?}", config_path);
            
            if config_path.exists() {
                println!("Config file exists");
                if let Ok(content) = std::fs::read_to_string(config_path) {
                    println!("Config content: {}", content);
                    
                    // 尝试手动解析配置文件
                    let mut config = Self::default();
                    let lines = content.lines();
                    
                    for line in lines {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }
                        
                        if let Some((key, value)) = line.split_once('=') {
                            let key = key.trim();
                            let value = value.trim();
                            
                            match key {
                                "selected_port" => {
                                    if value == "None" {
                                        config.selected_port = None;
                                    } else {
                                        // 移除引号
                                        let value = value.trim_matches('"');
                                        config.selected_port = Some(value.to_string());
                                    }
                                }
                                "baud_rate" => {
                                    if let Ok(v) = value.parse::<u32>() {
                                        config.baud_rate = v;
                                    }
                                }
                                "data_bits" => {
                                    if let Ok(v) = value.parse::<u8>() {
                                        config.data_bits = v;
                                    }
                                }
                                "parity" => {
                                    if let Ok(v) = value.parse::<u8>() {
                                        config.parity = v;
                                    }
                                }
                                "stop_bits" => {
                                    if let Ok(v) = value.parse::<u8>() {
                                        config.stop_bits = v;
                                    }
                                }
                                "display_mode" => {
                                    let value = value.trim_matches('"');
                                    config.display_mode = value.to_string();
                                }
                                "receive_encoding" => {
                                    let value = value.trim_matches('"');
                                    config.receive_encoding = value.to_string();
                                }
                                "should_auto_scroll" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.should_auto_scroll = v;
                                    }
                                }
                                "send_encoding" => {
                                    let value = value.trim_matches('"');
                                    config.send_encoding = value.to_string();
                                }
                                "send_newline" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.send_newline = v;
                                    }
                                }
                                "is_dark_mode" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.is_dark_mode = v;
                                    }
                                }
                                "show_settings_panel" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.show_settings_panel = v;
                                    }
                                }
                                "use_cloud_communication" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.use_cloud_communication = v;
                                    }
                                }
                                "cloud_service" => {
                                    let value = value.trim_matches('"');
                                    config.cloud_service = value.to_string();
                                }
                                "cloud_protocol" => {
                                    let value = value.trim_matches('"');
                                    config.cloud_protocol = value.to_string();
                                }
                                "cloud_uid" => {
                                    let value = value.trim_matches('"');
                                    config.cloud_uid = value.to_string();
                                }
                                "cloud_subscribe_topics" => {
                                    let value = value.trim_matches('"');
                                    config.cloud_subscribe_topics = value.split(',').map(|s| s.trim().to_string()).collect();
                                }
                                "cloud_publish_topics" => {
                                    let value = value.trim_matches('"');
                                    config.cloud_publish_topics = value.split(',').map(|s| s.trim().to_string()).collect();
                                }
                                "show_cloud_debug_info" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.show_cloud_debug_info = v;
                                    }
                                }
                                "dataflow_enabled" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.dataflow_enabled = v;
                                    }
                                }
                                "use_dedicated_firmware" => {
                                    if let Ok(v) = value.parse::<bool>() {
                                        config.use_dedicated_firmware = v;
                                    }
                                }
                                "shortcuts" => {
                                    // 处理 TOML 数组格式
                                    let value = value.trim();
                                    if value.starts_with('[') && value.ends_with(']') {
                                        // 移除 []
                                        let inner = value.trim_matches('[').trim_matches(']');
                                        // 正确解析 TOML 数组元素，处理包含逗号的情况
                                        let mut shortcuts = Vec::new();
                                        let mut current = String::new();
                                        let mut in_quotes = false;
                                        
                                        for c in inner.chars() {
                                            match c {
                                                '"' => {
                                                    in_quotes = !in_quotes;
                                                    current.push(c);
                                                }
                                                ',' if !in_quotes => {
                                                    // 遇到逗号且不在引号内，说明是元素分隔符
                                                    if !current.trim().is_empty() {
                                                        let shortcut = current.trim().trim_matches('"').to_string();
                                                        if !shortcut.is_empty() {
                                                            shortcuts.push(shortcut);
                                                        }
                                                    }
                                                    current.clear();
                                                }
                                                _ => {
                                                    current.push(c);
                                                }
                                            }
                                        }
                                        
                                        // 处理最后一个元素
                                        if !current.trim().is_empty() {
                                            let shortcut = current.trim().trim_matches('"').to_string();
                                            if !shortcut.is_empty() {
                                                shortcuts.push(shortcut);
                                            }
                                        }
                                        
                                        config.shortcuts = shortcuts;
                                    } else {
                                        // 兼容旧格式
                                        let value = value.trim_matches('"');
                                        config.shortcuts = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                                    }
                                }
                                // 兼容旧配置
                                "cloud_subscribe_topic" => {
                                    let value = value.trim_matches('"');
                                    if !value.is_empty() {
                                        config.cloud_subscribe_topics = vec![value.to_string()];
                                    }
                                }
                                "cloud_publish_topic" => {
                                    let value = value.trim_matches('"');
                                    if !value.is_empty() {
                                        config.cloud_publish_topics = vec![value.to_string()];
                                    }
                                }
                                "window_x" => {
                                    if value == "None" || value == "null" {
                                        config.window_x = None;
                                    } else if let Ok(v) = value.parse::<f32>() {
                                        config.window_x = Some(v);
                                    }
                                }
                                "window_y" => {
                                    if value == "None" || value == "null" {
                                        config.window_y = None;
                                    } else if let Ok(v) = value.parse::<f32>() {
                                        config.window_y = Some(v);
                                    }
                                }
                                "window_width" => {
                                    if let Ok(v) = value.parse::<f32>() {
                                        config.window_width = v;
                                    }
                                }
                                "window_height" => {
                                    if let Ok(v) = value.parse::<f32>() {
                                        config.window_height = v;
                                    }
                                }

                                _ => {}
                            }
                        }
                    }
                    
                    println!("Config loaded successfully (manual parsing)");
                    println!("Loaded config: window_x={:?}, window_y={:?}, window_width={}, window_height={}", 
                             config.window_x, config.window_y, config.window_width, config.window_height);
                    return config;
                } else {
                    println!("Failed to read config file");
                }
            } else {
                println!("Config file does not exist");
            }
        } else {
            println!("Failed to get project directories");
        }
        println!("Using default config");
        Self::default()
    }
    
    // 保存配置
    pub fn save(&self) {
        if let Some(proj_dirs) = ProjectDirs::from("com", "serialmonitor", "serial_monitor") {
            let config_dir = proj_dirs.config_dir();
            println!("Saving config to: {:?}", config_dir);
            
            if !config_dir.exists() {
                println!("Creating config directory");
                if let Ok(_) = std::fs::create_dir_all(config_dir) {
                    println!("Config directory created");
                } else {
                    println!("Failed to create config directory");
                }
            }
            
            let config_path = config_dir.join("config.toml");
            println!("Config file path: {:?}", config_path);
            
            // 手动构建配置内容，确保包含所有字段
            let mut content = String::new();
            // 正确保存 selected_port，确保格式与 load 方法一致
            if let Some(port) = &self.selected_port {
                content.push_str(&format!("selected_port = \"{}\"\n", port));
            } else {
                content.push_str("selected_port = None\n");
            }
            content.push_str(&format!("baud_rate = {}\n", self.baud_rate));
            content.push_str(&format!("data_bits = {}\n", self.data_bits));
            content.push_str(&format!("parity = {}\n", self.parity));
            content.push_str(&format!("stop_bits = {}\n", self.stop_bits));
            content.push_str(&format!("display_mode = {:?}\n", self.display_mode));
            content.push_str(&format!("receive_encoding = {:?}\n", self.receive_encoding));
            content.push_str(&format!("should_auto_scroll = {}\n", self.should_auto_scroll));
            content.push_str(&format!("send_encoding = {:?}\n", self.send_encoding));
            content.push_str(&format!("send_newline = {}\n", self.send_newline));
            content.push_str(&format!("is_dark_mode = {}\n", self.is_dark_mode));
            content.push_str(&format!("show_settings_panel = {}\n", self.show_settings_panel));
            content.push_str(&format!("use_cloud_communication = {}\n", self.use_cloud_communication));
            content.push_str(&format!("cloud_service = {:?}\n", self.cloud_service));
            content.push_str(&format!("cloud_protocol = {:?}\n", self.cloud_protocol));
            content.push_str(&format!("cloud_uid = {:?}\n", self.cloud_uid));
            content.push_str(&format!("cloud_subscribe_topics = {:?}\n", self.cloud_subscribe_topics.join(", ")));
            content.push_str(&format!("cloud_publish_topics = {:?}\n", self.cloud_publish_topics.join(", ")));
            content.push_str(&format!("show_cloud_debug_info = {}\n", self.show_cloud_debug_info));
            content.push_str(&format!("dataflow_enabled = {}\n", self.dataflow_enabled));
            content.push_str(&format!("use_dedicated_firmware = {}\n", self.use_dedicated_firmware));
            // 使用 TOML 数组格式保存快捷指令，支持包含逗号的指令
            content.push_str("shortcuts = [");
            for (i, shortcut) in self.shortcuts.iter().enumerate() {
                if i > 0 {
                    content.push_str(", ");
                }
                content.push_str(&format!("{:?}", shortcut));
            }
            content.push_str("]\n");
            
            // 正确处理 Option<f32> 类型
            if let Some(x) = self.window_x {
                content.push_str(&format!("window_x = {}\n", x));
            } else {
                content.push_str("window_x = null\n");
            }
            
            if let Some(y) = self.window_y {
                content.push_str(&format!("window_y = {}\n", y));
            } else {
                content.push_str("window_y = null\n");
            }
            
            content.push_str(&format!("window_width = {}
", self.window_width));
            content.push_str(&format!("window_height = {}
", self.window_height));
            
            println!("Config content to save: {}", content);
            
            if let Ok(_) = std::fs::write(config_path, content) {
                println!("Config saved successfully");
            } else {
                println!("Failed to write config file");
            }
        } else {
            println!("Failed to get project directories");
        }
    }
}
