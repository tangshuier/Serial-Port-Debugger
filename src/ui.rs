use eframe::egui;
use encoding_rs;
use serialport;
use crate::DisplayMode;

// UI相关功能
pub fn render_ui(ui: &mut egui::Ui, app: &mut crate::SerialMonitor) {
    // 更新窗口大小字段
    let available_size = ui.available_size();
    app.window_width = available_size.x;
    app.window_height = available_size.y;
    
    // 1. 最顶部：标题栏、更新图标和主题切换
    ui.horizontal(|ui| {
        ui.heading("串口调试助手");
        
        // 更新图标按钮 - 根据是否有更新显示不同颜色
        let update_icon = if app.update_available {
            egui::Button::new(egui::RichText::new("🔄").color(egui::Color32::GREEN))
        } else {
            egui::Button::new(egui::RichText::new("🔄").color(egui::Color32::GRAY))
        };
        
        if ui.add(update_icon).clicked() {
            // 显示更新信息窗口
            app.show_update_info_window = true;
        }
        
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            ui.checkbox(&mut app.is_dark_mode, "暗色模式");
        });
    });
    
    ui.add_space(10.0);
    
    // 2. 中间主体部分：固定布局
    ui.vertical(|ui| {
        // 发送区域固定高度
        let send_area_height = 120.0;
        
        // 计算中间内容区域可用高度
        let middle_available_height = available_size.y - send_area_height - 40.0; // 40是间距和分隔符的预留高度
        
        // 确保中间区域高度合理，不小于最小高度
        let middle_area_height = middle_available_height.max(200.0);
        
        // 中间内容区域
        ui.horizontal(|ui| {
            // 左侧：接收数据区域（垂直布局）
            ui.push_id("receive_area", |ui| {
                let left_width = (available_size.x * 0.6).max(400.0);
                ui.set_width(left_width);
                ui.set_max_height(middle_area_height);
                ui.vertical(|ui| {
                    // 接收数据控制栏
                    ui.horizontal(|ui| {
                        ui.label("接收数据:");
                        ui.checkbox(&mut app.should_auto_scroll, "自动滚动");
                        if ui.button("复制数据").clicked() {
                            // 使用egui提供的剪贴板功能
                            ui.output_mut(|o| o.copied_text = app.received_data.clone());
                        }
                        if ui.button("清空数据").clicked() {
                            app.received_data.clear();
                        }
                    });
                    
                    ui.separator();
                    
                    // 数据展示区域 - 填充剩余高度
                    let available_height = ui.available_size().y;
                    egui::ScrollArea::vertical()
                        .id_source("receive_scroll")
                        .auto_shrink([false; 2])
                        .max_height(available_height - 10.0)
                        .show(ui, |ui| {
                            // 使用Label::new并设置wrap来确保文本自动换行
                            ui.add(egui::Label::new(&app.received_data).wrap(true));
                            if app.should_auto_scroll {
                                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            }
                        });
                });
            });
            
            ui.add_space(10.0);
            
            // 右侧：设置区域（垂直布局）
            ui.push_id("config_area", |ui| {
                let right_width = (available_size.x * 0.4 - 10.0).max(250.0);
                ui.set_width(right_width);
                ui.set_max_height(middle_area_height);
                
                // 直接显示设置内容，不使用滚动区域
                ui.vertical(|ui| {
                    // 使用按钮组实现标签页效果
                    ui.horizontal(|ui| {
                        // 确定当前选中的标签
                        let is_serial_tab = app.current_tab == "串口设置";
                        let is_shortcut_tab = app.current_tab == "快捷指令";
                        let is_cloud_tab = app.current_tab == "云端通信";
                        let is_info_tab = app.current_tab == "资料";
                        
                        // 串口设置标签
                        if ui.selectable_label(is_serial_tab, "串口设置").clicked() {
                            app.current_tab = "串口设置".to_string();
                        }
                        
                        // 快捷指令标签
                        if ui.selectable_label(is_shortcut_tab, "快捷指令").clicked() {
                            app.current_tab = "快捷指令".to_string();
                        }
                        
                        // 云端通信标签
                        if ui.selectable_label(is_cloud_tab, "云端通信").clicked() {
                            app.current_tab = "云端通信".to_string();
                        }
                        
                        // 资料标签
                        if ui.selectable_label(is_info_tab, "资料").clicked() {
                            app.current_tab = "资料".to_string();
                        }
                    });
                    
                    ui.separator();
                    
                    // 根据当前选中的标签显示对应的内容
                    match app.current_tab.as_str() {
                        "串口设置" => {
                            // 原有设置
                            // 串口设置
                            ui.heading("串口设置");
                            render_serial_settings(ui, app);
                            
                            ui.add_space(10.0);
                            
                            // 显示设置
                            ui.heading("显示设置");
                            render_display_settings(ui, app);
                        }
                        "快捷指令" => {
                            // 快捷指令标签页内容
                            ui.heading("快捷指令");
                            
                            // 添加指令按钮
                            if ui.button("添加指令").clicked() {
                                app.new_shortcut = String::new();
                                app.editing_shortcut_index = None;
                                app.show_shortcut_window = true;
                            }
                            
                            ui.add_space(10.0);
                            
                            // 快捷指令列表 - 使用滚动区域
                            egui::ScrollArea::vertical()
                                .id_source("shortcuts_scroll")
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    if app.shortcuts.is_empty() {
                                        ui.label("暂无快捷指令，点击添加指令按钮添加");
                                    } else {
                                        // 遍历快捷指令列表
                                        let mut indices_to_remove = Vec::new();
                                        for (i, (shortcut, include_newline)) in app.shortcuts.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                // 计算可用宽度
                                                let available_width = ui.available_width();
                                                let label_width = available_width * 0.4; // 指令文本占用40%宽度
                                                
                                                // 显示指令内容，设置最大宽度
                                                ui.push_id(format!("shortcut_label_{}", i), |ui| {
                                                    ui.set_width(label_width);
                                                    ui.add(egui::Label::new(shortcut).wrap(true).truncate(true));
                                                });
                                                
                                                // 按钮区域
                                                ui.push_id(format!("shortcut_buttons_{}", i), |ui| {
                                                    // 发送按钮
                                                if ui.button("发送").clicked() {
                                                    // 发送指令到串口
                                                    let mut data_to_send = Vec::new();
                                                    let mut display_text = String::new();
                                                    
                                                    if app.send_hex {
                                                        // 十六进制发送模式
                                                        let hex_str = shortcut.replace(" ", "");
                                                        if hex_str.len() % 2 == 0 {
                                                            for i in (0..hex_str.len()).step_by(2) {
                                                                if let Ok(byte) = u8::from_str_radix(&hex_str[i..i+2], 16) {
                                                                    data_to_send.push(byte);
                                                                } else {
                                                                    app.received_data.push_str("发送错误: 无效的十六进制格式\n");
                                                                    data_to_send.clear();
                                                                    break;
                                                                }
                                                            }
                                                            if !data_to_send.is_empty() {
                                                                // 显示十六进制格式
                                                                display_text = hex_str.chars().collect::<Vec<char>>().chunks(2).map(|c| c.iter().collect::<String>()).collect::<Vec<String>>().join(" ");
                                                                if *include_newline {
                                                                    display_text.push_str("\n");
                                                                }
                                                                // 添加上传提示
                                                                display_text = format!("发送: {}", display_text);
                                                            }
                                                        } else {
                                                            app.received_data.push_str("发送错误: 十六进制字符串长度必须为偶数\n");
                                                            data_to_send.clear();
                                                        }
                                                    } else {
                                                        // 普通字符串发送模式
                                                        data_to_send.extend_from_slice(shortcut.as_bytes());
                                                        
                                                        // 添加换行符
                                                        if *include_newline {
                                                            data_to_send.extend_from_slice(&[13, 10]); // CRLF
                                                        }
                                                        
                                                        // 显示普通文本格式
                                                        display_text = if *include_newline {
                                                            format!("发送: {}\n", shortcut)
                                                        } else {
                                                            format!("发送: {}", shortcut)
                                                        };
                                                    }
                                                    
                                                    // 发送数据
                                                    if !data_to_send.is_empty() {
                                                        if let Err(e) = app.serial_manager.send_data(&data_to_send) {
                                                            app.received_data.push_str(&format!("发送错误: {}\n", e));
                                                        } else {
                                                            // 在接收区域显示发送的数据
                                                            let timestamp = if app.show_timestamp {
                                                                let now = chrono::Local::now();
                                                                format!("[{}] ", now.format("%Y-%m-%d %H:%M:%S"))
                                                            } else {
                                                                String::new()
                                                            };
                                                            app.received_data.push_str(&format!("{}{}", timestamp, display_text));
                                                        }
                                                    }
                                                }
                                                    
                                                    // 编辑按钮
                                                    if ui.button("编辑").clicked() {
                                                        app.new_shortcut = shortcut.clone();
                                                        app.new_shortcut_newline = *include_newline;
                                                        app.editing_shortcut_index = Some(i);
                                                        app.show_shortcut_window = true;
                                                    }
                                                    
                                                    // 删除按钮
                                                    if ui.button("删除").clicked() {
                                                        indices_to_remove.push(i);
                                                    }
                                                });
                                            });
                                            ui.add_space(5.0);
                                        }
                                        
                                        // 从后往前删除，避免索引偏移
                                        for &i in indices_to_remove.iter().rev() {
                                            app.shortcuts.remove(i);
                                            app.save_config();
                                        }
                                    }
                                });
                        }
                        "云端通信" => {
                            // 云端通信设置
                            ui.heading("云端通信设置");
                            ui.horizontal(|ui| {
                                // 配置按钮
                                if ui.button("配置").clicked() {
                                    app.show_cloud_config_window = true;
                                }
                                ui.add_space(5.0);
                                // 连接/断开按钮，根据当前连接状态显示
                                if ui.button(if app.cloud_manager.connected { "断开云端服务器" } else { "连接云端服务器" }).clicked() {
                                    if app.cloud_manager.connected {
                                        app.cloud_manager.disconnect();
                                        if app.cloud_manager.show_debug_info {
                                            app.received_data.push_str("云端连接已断开\n");
                                        }
                                    } else {
                                        // 调用连接方法
                                        match app.cloud_manager.connect_to_bemfa() {
                                            Ok(_) => {
                                                if app.cloud_manager.show_debug_info {
                                                    app.received_data.push_str("云端连接成功\n");
                                                }
                                            }
                                            Err(e) => {
                                                if app.cloud_manager.show_debug_info {
                                                    app.received_data.push_str(&format!("云端连接失败: {}\n", e));
                                                }
                                            }
                                        }
                                    }
                                }
                                ui.add_space(5.0);
                                // 云端调试信息显示开关
                                ui.checkbox(&mut app.cloud_manager.show_debug_info, "显示调试信息");
                            });
                            ui.add_space(5.0);
                            // 连接状态
                            ui.label(format!("云端连接状态: {}", if app.cloud_manager.connected { "已连接" } else { "未连接" }));
                            // 显示当前配置信息
                            ui.add_space(5.0);
                            ui.label(format!("当前云服务: {}", app.cloud_manager.service));
                            ui.label(format!("当前协议: {}", app.cloud_manager.protocol));
                            let subscribe_topics_str = if app.cloud_manager.subscribe_topics.is_empty() { "无".to_string() } else { app.cloud_manager.subscribe_topics.join(", ") };
                            ui.label(format!("订阅主题: {}", subscribe_topics_str));
                            let publish_topics_str = if app.cloud_manager.publish_topics.is_empty() { "无".to_string() } else { app.cloud_manager.publish_topics.join(", ") };
                            ui.label(format!("上传主题: {}", publish_topics_str));
                            
                            ui.add_space(10.0);
                            
                            // 数据流转设置
                            ui.horizontal(|ui| {
                                ui.heading("数据流转设置");
                                // 添加带有工具提示的问号图标，设置为小按钮
                                let _ = ui.add(egui::Button::new("?").small()).on_hover_text("数据流转:\n串口↔云端双向传输\n不同连接模式\n有不同打包格式");
                            });
                            ui.checkbox(&mut app.dataflow_manager.enabled, "启用数据流转");
                            
                            ui.add_space(10.0);
                            
                            // 连接模式设置
                            ui.heading("连接模式设置");
                            ui.push_id("connection_mode_combo", |ui| {
                                egui::ComboBox::from_label("选择模式")
                                    .selected_text(match app.dataflow_manager.connection_mode {
                                        crate::dataflow::ConnectionMode::Direct => "直连模式",
                                        crate::dataflow::ConnectionMode::Firmware => "固件模式",
                                        crate::dataflow::ConnectionMode::AT => "AT模式",
                                    })
                                    .width(120.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut app.dataflow_manager.connection_mode,
                                            crate::dataflow::ConnectionMode::Direct,
                                            "直连模式"
                                        ).on_hover_text("直接连接到串口设备，支持实时数据传输，当前默认模式");
                                        ui.selectable_value(
                                            &mut app.dataflow_manager.connection_mode,
                                            crate::dataflow::ConnectionMode::Firmware,
                                            "固件模式"
                                        ).on_hover_text("适用于专用固件设备，支持JSON格式数据传输");
                                        ui.selectable_value(
                                            &mut app.dataflow_manager.connection_mode,
                                            crate::dataflow::ConnectionMode::AT,
                                            "AT模式"
                                        ).on_hover_text("适用于AT指令控制的设备，支持AT指令集操作");
                                    });
                            });
                        }
                        "资料" => {
                            // 资料标签页内容
                            ui.heading("资料");
                            ui.label("资料列表");
                            
                            ui.add_space(10.0);
                            
                            // 专有固件下载按钮
                            if ui.button("专有固件").on_hover_text("借助专用ESP8266-01S固件包实现的云端连接程序").clicked() {
                                // 实现下载专有固件程序的功能
                                // 弹出文件保存对话框
                                let mut dir_path = None;
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("选择保存专有固件程序的文件夹")
                                    .pick_folder() {
                                    dir_path = Some(path);
                                }
                                
                                if let Some(path) = dir_path {
                                    ui.label(&format!("正在下载专有固件程序到: {}", path.display()));
                                    
                                    // 实际的下载功能：复制专有固件程序文件夹到用户指定的位置
                                    let source_dir = std::path::Path::new("专有固件程序");
                                    let target_dir = path.join("专有固件程序");
                                    
                                    // 创建目标目录
                                    if let Err(e) = std::fs::create_dir_all(&target_dir) {
                                        ui.label(&format!("创建目录失败: {}", e));
                                    } else {
                                        // 复制目录中的所有文件
                                        if let Ok(entries) = std::fs::read_dir(source_dir) {
                                            for entry in entries {
                                                if let Ok(entry) = entry {
                                                    let source_file = entry.path();
                                                    let file_name = source_file.file_name().unwrap();
                                                    let target_file = target_dir.join(file_name);
                                                    
                                                    if let Err(e) = std::fs::copy(&source_file, &target_file) {
                                                        ui.label(&format!("复制文件失败: {}", e));
                                                    }
                                                }
                                            }
                                            ui.label("下载完成");
                                        } else {
                                            ui.label("读取源目录失败");
                                        }
                                    }
                                } else {
                                    ui.label("下载已取消");
                                }
                            }
                            
                            ui.add_space(5.0);
                            
                            // AT固件下载按钮
                            if ui.button("AT固件").on_hover_text("借助官方AT指令实现的云端连接程序").clicked() {
                                // 实现下载AT固件程序的功能
                                // 弹出文件保存对话框
                                let mut dir_path = None;
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("选择保存AT固件程序的文件夹")
                                    .pick_folder() {
                                    dir_path = Some(path);
                                }
                                
                                if let Some(path) = dir_path {
                                    ui.label(&format!("正在下载AT固件程序到: {}", path.display()));
                                    
                                    // 实际的下载功能：复制AT固件程序文件夹到用户指定的位置
                                    let source_dir = std::path::Path::new("AT固件程序");
                                    let target_dir = path.join("AT固件程序");
                                    
                                    // 创建目标目录
                                    if let Err(e) = std::fs::create_dir_all(&target_dir) {
                                        ui.label(&format!("创建目录失败: {}", e));
                                    } else {
                                        // 复制目录中的所有文件
                                        if let Ok(entries) = std::fs::read_dir(source_dir) {
                                            for entry in entries {
                                                if let Ok(entry) = entry {
                                                    let source_file = entry.path();
                                                    let file_name = source_file.file_name().unwrap();
                                                    let target_file = target_dir.join(file_name);
                                                    
                                                    if let Err(e) = std::fs::copy(&source_file, &target_file) {
                                                        ui.label(&format!("复制文件失败: {}", e));
                                                    }
                                                }
                                            }
                                            ui.label("下载完成");
                                        } else {
                                            ui.label("读取源目录失败");
                                        }
                                    }
                                } else {
                                    ui.label("下载已取消");
                                }
                            }
                        }
                        _ => {
                            // 默认显示云端通信设置
                            ui.heading("云端通信设置");
                            ui.label("请配置云端通信信息");
                        }
                    }
                });
            });
        });
        
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        
        // 底部：下发数据区域
        ui.push_id("send_area", |ui| {
            ui.set_max_height(send_area_height);
            ui.label("下发数据:");
            
            ui.add_space(4.0);
            
            // 输入框和发送按钮
            ui.horizontal(|ui| {
                // 输入框占据大部分空间
                ui.text_edit_singleline(&mut app.send_data);
                
                // 发送按钮
                if ui.button("发送").clicked() && !app.send_data.is_empty() {
                    // 根据选择的编码格式处理数据
                    let mut data_to_send = Vec::new();
                    let mut display_text = String::new();
                    
                    if app.send_hex {
                        // 十六进制发送模式
                        let hex_str = app.send_data.replace(" ", "");
                        if hex_str.len() % 2 == 0 {
                            for i in (0..hex_str.len()).step_by(2) {
                                if let Ok(byte) = u8::from_str_radix(&hex_str[i..i+2], 16) {
                                    data_to_send.push(byte);
                                } else {
                                    app.received_data.push_str("发送错误: 无效的十六进制格式\n");
                                    data_to_send.clear();
                                    break;
                                }
                            }
                            if !data_to_send.is_empty() {
                                // 显示十六进制格式
                                display_text = hex_str.chars().collect::<Vec<char>>().chunks(2).map(|c| c.iter().collect::<String>()).collect::<Vec<String>>().join(" ");
                                if app.send_newline {
                                    display_text.push_str("\n");
                                }
                                // 添加上传提示
                                display_text = format!("发送: {}", display_text);
                            }
                        } else {
                            app.received_data.push_str("发送错误: 十六进制字符串长度必须为偶数\n");
                            data_to_send.clear();
                        }
                    } else {
                        // 普通字符串发送模式
                        match app.send_encoding.as_str() {
                            "UTF-8" => {
                                data_to_send.extend_from_slice(app.send_data.as_bytes());
                            }
                            "GB2312" => {
                                let (text, _, _) = encoding_rs::GBK.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "Big5" => {
                                let (text, _, _) = encoding_rs::BIG5.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "EUC-JP" => {
                                let (text, _, _) = encoding_rs::EUC_JP.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "Shift_JIS" => {
                                let (text, _, _) = encoding_rs::SHIFT_JIS.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "KOI8-R" => {
                                let (text, _, _) = encoding_rs::KOI8_R.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "Windows-1251" => {
                                let (text, _, _) = encoding_rs::WINDOWS_1251.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "Windows-1252" => {
                                let (text, _, _) = encoding_rs::WINDOWS_1252.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "UTF-16 LE" => {
                                let (text, _, _) = encoding_rs::UTF_16LE.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "UTF-16 BE" => {
                                let (text, _, _) = encoding_rs::UTF_16BE.encode(&app.send_data);
                                data_to_send.extend_from_slice(&text);
                            }
                            "Latin1" | "ASCII" => {
                                // Latin1 和 ASCII 编码：每个字符直接转换为对应的字节
                                for c in app.send_data.chars() {
                                    data_to_send.push(c as u8);
                                }
                            }
                            _ => {
                                data_to_send.extend_from_slice(app.send_data.as_bytes());
                            }
                        }
                        
                        // 添加换行符
                        if app.send_newline {
                            data_to_send.extend_from_slice(&[13, 10]); // CRLF
                        }
                        
                        // 显示普通文本格式
                        display_text = if app.send_newline {
                            format!("{}\n", app.send_data)
                        } else {
                            app.send_data.clone()
                        };
                    }
                    
                    // 使用serial_manager的send_data方法发送数据
                    if !data_to_send.is_empty() {
                        if let Err(e) = app.serial_manager.send_data(&data_to_send) {
                            app.received_data.push_str(&format!("发送错误: {}\n", e));
                        } else {
                            // 在接收区域显示发送的数据
                            let timestamp = if app.show_timestamp {
                                let now = chrono::Local::now();
                                format!("[{}] ", now.format("%Y-%m-%d %H:%M:%S"))
                            } else {
                                String::new()
                            };
                            app.received_data.push_str(&format!("{}{}", timestamp, display_text));
                        }
                    }
                }
            });
            
            ui.add_space(4.0);
            
            // 发送选项
            ui.horizontal(|ui| {
                // 编码选择
                ui.label("编码:");
                ui.push_id("send_encoding_combo", |ui| {
                    egui::ComboBox::from_label("")
                        .selected_text(&app.send_encoding)
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            ui.push_id("send_utf8", |ui| {
                                if ui.selectable_label(app.send_encoding == "UTF-8", "UTF-8").clicked() {
                                    app.send_encoding = "UTF-8".to_string();
                                }
                            });
                            ui.push_id("send_gb2312", |ui| {
                                if ui.selectable_label(app.send_encoding == "GB2312", "GB2312").clicked() {
                                    app.send_encoding = "GB2312".to_string();
                                }
                            });
                            ui.push_id("send_big5", |ui| {
                                if ui.selectable_label(app.send_encoding == "Big5", "Big5").clicked() {
                                    app.send_encoding = "Big5".to_string();
                                }
                            });
                            ui.push_id("send_euc_jp", |ui| {
                                if ui.selectable_label(app.send_encoding == "EUC-JP", "EUC-JP").clicked() {
                                    app.send_encoding = "EUC-JP".to_string();
                                }
                            });
                            ui.push_id("send_shift_jis", |ui| {
                                if ui.selectable_label(app.send_encoding == "Shift_JIS", "Shift_JIS").clicked() {
                                    app.send_encoding = "Shift_JIS".to_string();
                                }
                            });
                            ui.push_id("send_koi8_r", |ui| {
                                if ui.selectable_label(app.send_encoding == "KOI8-R", "KOI8-R").clicked() {
                                    app.send_encoding = "KOI8-R".to_string();
                                }
                            });
                            ui.push_id("send_windows_1251", |ui| {
                                if ui.selectable_label(app.send_encoding == "Windows-1251", "Windows-1251").clicked() {
                                    app.send_encoding = "Windows-1251".to_string();
                                }
                            });
                            ui.push_id("send_windows_1252", |ui| {
                                if ui.selectable_label(app.send_encoding == "Windows-1252", "Windows-1252").clicked() {
                                    app.send_encoding = "Windows-1252".to_string();
                                }
                            });
                            ui.push_id("send_utf16_le", |ui| {
                                if ui.selectable_label(app.send_encoding == "UTF-16 LE", "UTF-16 LE").clicked() {
                                    app.send_encoding = "UTF-16 LE".to_string();
                                }
                            });
                            ui.push_id("send_utf16_be", |ui| {
                                if ui.selectable_label(app.send_encoding == "UTF-16 BE", "UTF-16 BE").clicked() {
                                    app.send_encoding = "UTF-16 BE".to_string();
                                }
                            });
                            ui.push_id("send_latin1", |ui| {
                                if ui.selectable_label(app.send_encoding == "Latin1", "Latin1").clicked() {
                                    app.send_encoding = "Latin1".to_string();
                                }
                            });
                            ui.push_id("send_ascii", |ui| {
                                if ui.selectable_label(app.send_encoding == "ASCII", "ASCII").clicked() {
                                    app.send_encoding = "ASCII".to_string();
                                }
                            });
                        });
                });
                
                // 换行符选项
                ui.push_id("newline_checkbox", |ui| {
                    ui.checkbox(&mut app.send_newline, "包含换行符");
                });
                
                // 十六进制发送选项
                ui.push_id("hex_checkbox", |ui| {
                    ui.checkbox(&mut app.send_hex, "十六进制发送");
                });
            });
        });
    });
}

// 渲染云端配置窗口
pub fn render_cloud_config_window(ctx: &egui::Context, app: &mut crate::SerialMonitor) {
    if app.show_cloud_config_window {
        // 检查云端连接状态
        if app.cloud_manager.connected {
            // 云端已连接，显示提示信息
            egui::Window::new("配置提示")
                .resizable(false)
                .default_size([300.0, 150.0])
                .show(ctx, |ui| {
                    ui.label("请先断开云端连接再进行配置");
                    ui.add_space(20.0);
                    if ui.button("确定").clicked() {
                        app.show_cloud_config_window = false;
                    }
                });
        } else {
            // 云端未连接，显示配置窗口
            egui::Window::new("云端配置")
                .resizable(false)
                .default_size([350.0, 400.0])
                .show(ctx, |ui| {
                    // 云服务提供商选择
                    ui.label("云服务提供商:");
                    ui.push_id("cloud_service_combo", |ui| {
                        egui::ComboBox::from_label("")
                            .selected_text(&app.cloud_manager.service)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.push_id("cloud_service_bafayun", |ui| {
                                    if ui.selectable_label(app.cloud_manager.service == "巴法云", "巴法云").clicked() {
                                        app.cloud_manager.service = "巴法云".to_string();
                                    }
                                });
                                ui.push_id("cloud_service_onenet", |ui| {
                                    if ui.selectable_label(app.cloud_manager.service == "onenet云", "onenet云").clicked() {
                                        app.cloud_manager.service = "onenet云".to_string();
                                    }
                                });
                            });
                    });
                    ui.add_space(10.0);
                    
                    // 通讯协议选择
                    ui.label("通讯协议:");
                    ui.push_id("cloud_protocol_combo", |ui| {
                        egui::ComboBox::from_label("")
                            .selected_text(&app.cloud_manager.protocol)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                ui.push_id("cloud_protocol_tcp", |ui| {
                                    if ui.selectable_label(app.cloud_manager.protocol == "TCP", "TCP").clicked() {
                                        app.cloud_manager.protocol = "TCP".to_string();
                                    }
                                });
                                ui.push_id("cloud_protocol_mqtt", |ui| {
                                    if ui.selectable_label(app.cloud_manager.protocol == "MQTT", "MQTT").clicked() {
                                        app.cloud_manager.protocol = "MQTT".to_string();
                                    }
                                });
                                ui.push_id("cloud_protocol_storage", |ui| {
                                    if ui.selectable_label(app.cloud_manager.protocol == "图存储", "图存储").clicked() {
                                        app.cloud_manager.protocol = "图存储".to_string();
                                    }
                                });
                            });
                    });
                    ui.add_space(10.0);
                    
                    // 云端私钥
                    ui.label("云端私钥 (UID):");
                    ui.text_edit_singleline(&mut app.cloud_manager.uid);
                    ui.add_space(10.0);
                    
                    // 订阅主题
                    ui.label("订阅主题:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut app.new_subscribe_topic);
                        if ui.button("添加").clicked() {
                            // 验证输入内容
                            let trimmed_topic = app.new_subscribe_topic.trim();
                            if trimmed_topic.is_empty() {
                                // 显示错误提示窗口
                                app.error_message = "订阅主题不能为空".to_string();
                                app.show_error_window = true;
                            } else if trimmed_topic.contains(|c| c == ' ' || c == '\n' || c == '\r') {
                                // 显示错误提示窗口
                                app.error_message = "订阅主题不能包含空格、回车或换行符".to_string();
                                app.show_error_window = true;
                            } else if !app.cloud_manager.subscribe_topics.contains(&trimmed_topic.to_string()) {
                                // 检查是否已存在于上传主题列表中
                                if app.cloud_manager.publish_topics.contains(&trimmed_topic.to_string()) {
                                    // 显示错误提示窗口
                                    app.error_message = format!("订阅主题 '{}' 已存在于上传主题列表中，不能重复添加", trimmed_topic);
                                    app.show_error_window = true;
                                } else {
                                    // 输入内容有效且未重复，添加到主题列表
                                    app.cloud_manager.subscribe_topics.push(trimmed_topic.to_string());
                                    app.new_subscribe_topic.clear();
                                }
                            }
                        }
                    });
                    // 显示已添加的订阅主题
                    if !app.cloud_manager.subscribe_topics.is_empty() {
                        ui.label("已添加的订阅主题:");
                        // 创建一个临时的索引列表，避免在迭代时修改
                        let mut indices_to_remove = Vec::new();
                        for (i, topic) in app.cloud_manager.subscribe_topics.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(topic);
                                if ui.button("删除").clicked() {
                                    indices_to_remove.push(i);
                                }
                            });
                        }
                        // 从后往前删除，避免索引偏移
                        for &i in indices_to_remove.iter().rev() {
                            app.cloud_manager.subscribe_topics.remove(i);
                        }
                    }
                    ui.add_space(10.0);
                    
                    // 上传主题
                    ui.label("上传主题:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut app.new_publish_topic);
                        if ui.button("添加").clicked() {
                            // 验证输入内容
                            let trimmed_topic = app.new_publish_topic.trim();
                            if trimmed_topic.is_empty() {
                                // 显示错误提示窗口
                                app.error_message = "上传主题不能为空".to_string();
                                app.show_error_window = true;
                            } else if trimmed_topic.contains(|c| c == ' ' || c == '\n' || c == '\r') {
                                // 显示错误提示窗口
                                app.error_message = "上传主题不能包含空格、回车或换行符".to_string();
                                app.show_error_window = true;
                            } else if !app.cloud_manager.publish_topics.contains(&trimmed_topic.to_string()) {
                                // 检查是否已存在于订阅主题列表中
                                if app.cloud_manager.subscribe_topics.contains(&trimmed_topic.to_string()) {
                                    // 显示错误提示窗口
                                    app.error_message = format!("上传主题 '{}' 已存在于订阅主题列表中，不能重复添加", trimmed_topic);
                                    app.show_error_window = true;
                                } else {
                                    // 输入内容有效且未重复，添加到主题列表
                                    app.cloud_manager.publish_topics.push(trimmed_topic.to_string());
                                    app.new_publish_topic.clear();
                                }
                            }
                        }
                    });
                    // 显示已添加的上传主题
                    if !app.cloud_manager.publish_topics.is_empty() {
                        ui.label("已添加的上传主题:");
                        // 创建一个临时的索引列表，避免在迭代时修改
                        let mut indices_to_remove = Vec::new();
                        for (i, topic) in app.cloud_manager.publish_topics.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(topic);
                                if ui.button("删除").clicked() {
                                    indices_to_remove.push(i);
                                }
                            });
                        }
                        // 从后往前删除，避免索引偏移
                        for &i in indices_to_remove.iter().rev() {
                            app.cloud_manager.publish_topics.remove(i);
                        }
                    }
                    ui.add_space(20.0);
                    
                    // 底部按钮
                    ui.horizontal(|ui| {
                        if ui.button("确定").clicked() {
                            // 保存配置
                            app.save_config();
                            app.show_cloud_config_window = false;
                        }
                        if ui.button("取消").clicked() {
                            app.show_cloud_config_window = false;
                        }
                    });
                });
        }
    }
}

// 其余函数（render_serial_settings、render_display_settings）逻辑不变，无需修改
fn render_serial_settings(ui: &mut egui::Ui, app: &mut crate::SerialMonitor) {
    // 原有逻辑保持不变
    // 串口选择
    ui.push_id("serial_port_combo", |ui| {
        app.serial_manager.scan_ports();
        let available_ports = app.serial_manager.available_ports.clone();
        let selected_port = app.serial_manager.selected_port.clone();
        egui::ComboBox::from_label("选择串口")
            .selected_text(selected_port.as_deref().unwrap_or("未选择"))
            .show_ui(ui, |ui| {
                for (i, port) in available_ports.iter().enumerate() {
                    ui.push_id(format!("port_{}", i), |ui| {
                        if ui.selectable_label(
                            selected_port.as_deref() == Some(port),
                            port
                        ).clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.selected_port = Some(port.clone());
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                }
            });
    });
    
    // 连接控制
    ui.horizontal(|ui| {
        if !app.serial_manager.is_connected {
            if ui.button("连接").clicked() {
                match app.serial_manager.connect() {
                    Ok(_) => {
                        app.received_data.push_str("串口连接成功\n");
                    }
                    Err(e) => {
                        app.received_data.push_str(&format!("串口连接失败: {}\n", e));
                    }
                }
            }
        } else {
            if ui.button("断开").clicked() {
                app.serial_manager.disconnect();
                app.received_data.push_str("串口连接已断开\n");
            }
        }
        ui.label(format!("状态: {}", if app.serial_manager.is_connected { "已连接" } else { "未连接" }));
    });
    
    ui.add_space(10.0);
    
    // 串口参数设置
    ui.horizontal(|ui| {
        ui.label("波特率:");
        ui.push_id("baud_rate_combo", |ui| {
            let baud_rate = app.serial_manager.baud_rate;
            let baud_rates = app.serial_manager.baud_rates.clone();
            egui::ComboBox::from_label("")
                .selected_text(&format!("{}", baud_rate))
                .width(80.0)
                .show_ui(ui, |ui| {
                    for (i, rate) in baud_rates.iter().enumerate() {
                        ui.push_id(format!("rate_{}", i), |ui| {
                            if ui.selectable_label(baud_rate == *rate, &format!("{}", rate)).clicked() {
                                let was_connected = app.serial_manager.is_connected;
                                app.serial_manager.baud_rate = *rate;
                                if was_connected {
                                    app.serial_manager.disconnect();
                                    app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                    match app.serial_manager.connect() {
                                        Ok(_) => {
                                            app.received_data.push_str("串口重新连接成功\n");
                                        }
                                        Err(e) => {
                                            app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                        }
                                    }
                                }
                            }
                        });
                    }
                });
        });
    });
    
    ui.horizontal(|ui| {
        ui.label("数据位:");
        ui.push_id("data_bits_combo", |ui| {
            let data_bits = app.serial_manager.data_bits;
            egui::ComboBox::from_label("")
                .selected_text(match data_bits {
                    serialport::DataBits::Five => "5",
                    serialport::DataBits::Six => "6",
                    serialport::DataBits::Seven => "7",
                    serialport::DataBits::Eight => "8",
                })
                .width(60.0)
                .show_ui(ui, |ui| {
                    ui.push_id("data_bits_5", |ui| {
                        if ui.selectable_label(data_bits == serialport::DataBits::Five, "5").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.data_bits = serialport::DataBits::Five;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                    ui.push_id("data_bits_6", |ui| {
                        if ui.selectable_label(data_bits == serialport::DataBits::Six, "6").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.data_bits = serialport::DataBits::Six;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                    ui.push_id("data_bits_7", |ui| {
                        if ui.selectable_label(data_bits == serialport::DataBits::Seven, "7").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.data_bits = serialport::DataBits::Seven;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                    ui.push_id("data_bits_8", |ui| {
                        if ui.selectable_label(data_bits == serialport::DataBits::Eight, "8").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.data_bits = serialport::DataBits::Eight;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                });
        });
    });
    
    ui.horizontal(|ui| {
        ui.label("校验位:");
        ui.push_id("parity_combo", |ui| {
            let parity = app.serial_manager.parity;
            egui::ComboBox::from_label("")
                .selected_text(match parity {
                    serialport::Parity::None => "无",
                    serialport::Parity::Odd => "奇",
                    serialport::Parity::Even => "偶",
                })
                .width(60.0)
                .show_ui(ui, |ui| {
                    ui.push_id("parity_none", |ui| {
                        if ui.selectable_label(parity == serialport::Parity::None, "无").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.parity = serialport::Parity::None;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                    ui.push_id("parity_odd", |ui| {
                        if ui.selectable_label(parity == serialport::Parity::Odd, "奇").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.parity = serialport::Parity::Odd;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                    ui.push_id("parity_even", |ui| {
                        if ui.selectable_label(parity == serialport::Parity::Even, "偶").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.parity = serialport::Parity::Even;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                });
        });
    });
    
    ui.horizontal(|ui| {
        ui.label("停止位:");
        ui.push_id("stop_bits_combo", |ui| {
            let stop_bits = app.serial_manager.stop_bits;
            egui::ComboBox::from_label("")
                .selected_text(match stop_bits {
                    serialport::StopBits::One => "1",
                    serialport::StopBits::Two => "2",
                })
                .width(60.0)
                .show_ui(ui, |ui| {
                    ui.push_id("stop_bits_1", |ui| {
                        if ui.selectable_label(stop_bits == serialport::StopBits::One, "1").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.stop_bits = serialport::StopBits::One;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                    ui.push_id("stop_bits_2", |ui| {
                        if ui.selectable_label(stop_bits == serialport::StopBits::Two, "2").clicked() {
                            let was_connected = app.serial_manager.is_connected;
                            app.serial_manager.stop_bits = serialport::StopBits::Two;
                            if was_connected {
                                app.serial_manager.disconnect();
                                app.received_data.push_str("串口参数已修改，正在重新连接...\n");
                                match app.serial_manager.connect() {
                                    Ok(_) => {
                                        app.received_data.push_str("串口重新连接成功\n");
                                    }
                                    Err(e) => {
                                        app.received_data.push_str(&format!("串口重新连接失败: {}\n", e));
                                    }
                                }
                            }
                        }
                    });
                });
        });
    });
}

// 渲染错误提示窗口
pub fn render_error_window(ctx: &egui::Context, app: &mut crate::SerialMonitor) {
    if app.show_error_window {
        let window_title = if app.error_message.contains("更新成功") {
            "更新完成"
        } else if app.error_message.contains("当前已是最新版本") {
            "更新提示"
        } else {
            "错误提示"
        };
        
        egui::Window::new(window_title)
            .resizable(false)
            .default_size([350.0, 150.0])
            .show(ctx, |ui| {
                ui.label(&app.error_message);
                
                if app.error_message.contains("更新成功") {
                    if ui.button("重启").clicked() {
                        app.show_error_window = false;
                        app.error_message.clear();
                        // 触发重启操作
                        app.restart_needed = true;
                    }
                } else if app.error_message.contains("当前已是最新版本") {
                    ui.horizontal(|ui| {
                        if ui.button("确定").clicked() {
                            app.show_error_window = false;
                            app.error_message.clear();
                        }
                        if ui.button("更多版本").clicked() {
                            app.show_error_window = false;
                            app.error_message.clear();
                            // 显示版本列表窗口
                            app.show_versions_window = true;
                            app.is_loading_versions = true;
                            
                            // 在后台线程中获取所有版本
                            let ctx = ctx.clone();
                            std::thread::spawn(move || {
                                match crate::update::get_all_versions() {
                                    Ok(versions) => {
                                        // 更新版本列表
                                        *crate::VERSIONS.lock().unwrap() = versions;
                                        // 通知主线程
                                        ctx.request_repaint();
                                        crate::VERSIONS_LOADED.store(true, std::sync::atomic::Ordering::Relaxed);
                                    },
                                    Err(e) => {
                                        println!("获取版本列表失败: {:?}", e);
                                    }
                                }
                            });
                        }
                    });
                } else {
                    if ui.button("确定").clicked() {
                        app.show_error_window = false;
                        app.error_message.clear();
                    }
                }
            });
    }
}

// 渲染快捷指令编辑窗口
pub fn render_shortcut_window(ctx: &egui::Context, app: &mut crate::SerialMonitor) {
    if app.show_shortcut_window {
        egui::Window::new(if app.editing_shortcut_index.is_some() { "编辑指令" } else { "添加指令" })
            .resizable(false)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                ui.label("指令内容:");
                ui.text_edit_multiline(&mut app.new_shortcut);
                
                // 是否包含换行符选项
                ui.checkbox(&mut app.new_shortcut_newline, "包含换行符");
                
                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        if !app.new_shortcut.trim().is_empty() {
                            match app.editing_shortcut_index {
                                Some(index) => {
                                    // 编辑现有指令
                                    app.shortcuts[index] = (app.new_shortcut.trim().to_string(), app.new_shortcut_newline);
                                }
                                None => {
                                    // 添加新指令
                                    app.shortcuts.push((app.new_shortcut.trim().to_string(), app.new_shortcut_newline));
                                }
                            }
                            app.save_config();
                            app.show_shortcut_window = false;
                        } else {
                            app.error_message = "指令内容不能为空".to_string();
                            app.show_error_window = true;
                        }
                    }
                    if ui.button("取消").clicked() {
                        app.show_shortcut_window = false;
                    }
                });
            });
    }
}

fn render_display_settings(ui: &mut egui::Ui, app: &mut crate::SerialMonitor) {
    // 原有逻辑保持不变
    // 显示模式选择
    ui.horizontal(|ui| {
        ui.label("显示模式:");
        ui.push_id("display_mode_utf8", |ui| {
            if ui.radio(app.display_mode == DisplayMode::UTF8, "UTF-8").clicked() {
                app.display_mode = DisplayMode::UTF8;
            }
        });
        ui.push_id("display_mode_hex", |ui| {
            if ui.radio(app.display_mode == DisplayMode::Hex, "十六进制").clicked() {
                app.display_mode = DisplayMode::Hex;
            }
        });
        ui.push_id("display_mode_binary", |ui| {
            if ui.radio(app.display_mode == DisplayMode::Binary, "二进制").clicked() {
                app.display_mode = DisplayMode::Binary;
            }
        });
    });
    
    ui.add_space(10.0);
    
    // 时间戳设置
    ui.horizontal(|ui| {
        ui.label("时间戳:");
        ui.checkbox(&mut app.show_timestamp, "显示时间戳");
    });
    
    ui.add_space(10.0);
    
    // 接收编码选择
    ui.horizontal(|ui| {
        ui.label("接收编码:");
        ui.push_id("receive_encoding_combo", |ui| {
            egui::ComboBox::from_label("")
                .selected_text(&app.receive_encoding)
                .width(120.0)
                .show_ui(ui, |ui| {
                    ui.push_id("encoding_auto", |ui| {
                        if ui.selectable_label(app.receive_encoding == "自动识别", "自动识别").clicked() {
                            app.receive_encoding = "自动识别".to_string();
                        }
                    });
                    ui.push_id("encoding_utf8", |ui| {
                        if ui.selectable_label(app.receive_encoding == "UTF-8", "UTF-8").clicked() {
                            app.receive_encoding = "UTF-8".to_string();
                        }
                    });
                    ui.push_id("encoding_gb2312", |ui| {
                        if ui.selectable_label(app.receive_encoding == "GB2312", "GB2312").clicked() {
                            app.receive_encoding = "GB2312".to_string();
                        }
                    });
                    ui.push_id("encoding_big5", |ui| {
                        if ui.selectable_label(app.receive_encoding == "Big5", "Big5").clicked() {
                            app.receive_encoding = "Big5".to_string();
                        }
                    });
                    ui.push_id("encoding_euc_jp", |ui| {
                        if ui.selectable_label(app.receive_encoding == "EUC-JP", "EUC-JP").clicked() {
                            app.receive_encoding = "EUC-JP".to_string();
                        }
                    });
                    ui.push_id("encoding_shift_jis", |ui| {
                        if ui.selectable_label(app.receive_encoding == "Shift_JIS", "Shift_JIS").clicked() {
                            app.receive_encoding = "Shift_JIS".to_string();
                        }
                    });
                });
        });
    });
}