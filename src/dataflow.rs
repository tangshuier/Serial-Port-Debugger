use std::sync::{Arc, Mutex};
use serialport::SerialPort;
use crate::cloud::CloudManager;

// 连接模式枚举
#[derive(PartialEq, Copy, Clone)]
pub enum ConnectionMode {
    // 直连模式（当前实现）
    Direct,
    // 固件模式
    Firmware,
    // AT模式
    AT,
}

// 数据流转管理器
pub struct DataflowManager {
    // 数据流转启用状态
    pub enabled: bool,
    // 连接模式
    pub connection_mode: ConnectionMode,
}

impl Default for DataflowManager {
    fn default() -> Self {
        Self {
            enabled: false,
            connection_mode: ConnectionMode::Direct,
        }
    }
}

impl DataflowManager {
    // 从配置创建实例
    pub fn from_config(enabled: bool, connection_mode: ConnectionMode) -> Self {
        Self {
            enabled,
            connection_mode,
        }
    }
    
    // 处理串口数据上传到云端
    pub fn process_serial_to_cloud(
        &self,
        data: &str,
        cloud_manager: &mut CloudManager
    ) -> Result<(), String> {
        if !self.enabled || !cloud_manager.connected {
            return Ok(());
        }
        
        let upload_text = data.trim();
        if upload_text.is_empty() {
            return Ok(());
        }
        
        // 检查是否为WiFi模块控制消息
        if !self.is_wifi_module_control_message(upload_text) {
            // 根据云端服务、协议类型和连接模式处理数据
            match (cloud_manager.service.as_str(), cloud_manager.protocol.as_str(), self.connection_mode) {
                // 巴法云TCP协议处理
                ("巴法云", "TCP", ConnectionMode::Direct) => {
                    // 巴法云TCP直连模式：当前实现的核心功能
                    if let Some(topic) = self.extract_topic_from_serial_data(upload_text) {
                        if cloud_manager.publish_topics.contains(&topic) {
                            let msg_content = self.extract_msg_from_serial_data(upload_text)
                                .unwrap_or_else(|| upload_text.to_string());
                            cloud_manager.publish_data(&msg_content)?
                        }
                    }
                }
                ("巴法云", "TCP", ConnectionMode::Firmware) => {
                    // 巴法云TCP固件模式：预留位置，等待后续开发
                    // 暂时使用直连模式的逻辑
                    if let Some(topic) = self.extract_topic_from_serial_data(upload_text) {
                        if cloud_manager.publish_topics.contains(&topic) {
                            let msg_content = self.extract_msg_from_serial_data(upload_text)
                                .unwrap_or_else(|| upload_text.to_string());
                            cloud_manager.publish_data(&msg_content)?
                        }
                    }
                }
                ("巴法云", "TCP", ConnectionMode::AT) => {
                    // 巴法云TCP AT模式：预留位置，等待后续开发
                    // 暂时使用直连模式的逻辑
                    if let Some(topic) = self.extract_topic_from_serial_data(upload_text) {
                        if cloud_manager.publish_topics.contains(&topic) {
                            let msg_content = self.extract_msg_from_serial_data(upload_text)
                                .unwrap_or_else(|| upload_text.to_string());
                            cloud_manager.publish_data(&msg_content)?
                        }
                    }
                }
                // OneNet MQTT协议处理（预留）
                ("onenet云", "MQTT", _) => {
                    // 根据连接模式处理
                    if let Some(topic) = self.extract_topic_from_serial_data(upload_text) {
                        if cloud_manager.publish_topics.contains(&topic) {
                            let msg_content = self.extract_msg_from_serial_data(upload_text)
                                .unwrap_or_else(|| upload_text.to_string());
                            cloud_manager.publish_data(&msg_content)?
                        }
                    }
                }
                // OneNet HTTP协议处理（预留）
                ("onenet云", "HTTP", _) => {
                    // 类似处理逻辑
                    if let Some(topic) = self.extract_topic_from_serial_data(upload_text) {
                        if cloud_manager.publish_topics.contains(&topic) {
                            let msg_content = self.extract_msg_from_serial_data(upload_text)
                                .unwrap_or_else(|| upload_text.to_string());
                            cloud_manager.publish_data(&msg_content)?
                        }
                    }
                }
                // 其他情况的默认处理
                _ => {
                    // 使用默认处理逻辑
                    if let Some(topic) = self.extract_topic_from_serial_data(upload_text) {
                        if cloud_manager.publish_topics.contains(&topic) {
                            let msg_content = self.extract_msg_from_serial_data(upload_text)
                                .unwrap_or_else(|| upload_text.to_string());
                            cloud_manager.publish_data(&msg_content)?
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    // 处理云端数据下发到串口
    pub fn process_cloud_to_serial(
        &self,
        data: &str,
        port: &Option<Arc<Mutex<Box<dyn SerialPort>>>>,
        cloud_manager: &CloudManager
    ) -> Result<Option<String>, String> {
        if !self.enabled {
            return Ok(None);
        }
        
        // 实际发送的数据
        let mut actual_sent_data: Option<String> = None;
        
        // 根据连接模式处理数据
        match self.connection_mode {
            ConnectionMode::Direct => {
                // 直连模式：直接完整流转原始数据
                actual_sent_data = Some(data.to_string());
            }
            ConnectionMode::Firmware => {
                // 固件模式：构建与WiFi固件兼容的JSON格式
                // 尝试提取消息内容
                if let Some((topic, msg_content)) = cloud_manager.extract_message_from_response(data) {
                    let json_format = format!("{{\"topic\":\"{}\",\"payload\":{{\"data\":\"{}\"}}}}", topic, msg_content);
                    actual_sent_data = Some(json_format);
                } else {
                    // 如果提取失败，尝试手动解析
                    if data.starts_with("cmd=2&") {
                        // 手动解析格式：cmd=2&uid=xxx&topic=xxx&msg=xxx
                        let parts: Vec<&str> = data.split('&').collect();
                        let mut topic = "".to_string();
                        let mut msg = "".to_string();
                        
                        for part in parts {
                            if part.starts_with("topic=") {
                                topic = part.trim_start_matches("topic=").trim().to_string();
                            } else if part.starts_with("msg=") {
                                msg = part.trim_start_matches("msg=").trim().to_string();
                            }
                        }
                        
                        if !topic.is_empty() && !msg.is_empty() {
                            let json_format = format!("{{\"topic\":\"{}\",\"payload\":{{\"data\":\"{}\"}}}}", topic, msg);
                            actual_sent_data = Some(json_format);
                        }
                    }
                }
            }
            ConnectionMode::AT => {
                // AT模式：构建AT指令格式
                // 尝试提取消息内容
                if let Some((_, msg_content)) = cloud_manager.extract_message_from_response(data) {
                    let at_command = format!("AT+SEND={}\r\n", msg_content);
                    actual_sent_data = Some(at_command);
                } else {
                    // 如果提取失败，尝试手动解析
                    if data.starts_with("cmd=2&") {
                        // 手动解析格式：cmd=2&uid=xxx&topic=xxx&msg=xxx
                        let parts: Vec<&str> = data.split('&').collect();
                        let mut msg = "".to_string();
                        
                        for part in parts {
                            if part.starts_with("msg=") {
                                msg = part.trim_start_matches("msg=").trim().to_string();
                                break;
                            }
                        }
                        
                        if !msg.is_empty() {
                            let at_command = format!("AT+SEND={}\r\n", msg);
                            actual_sent_data = Some(at_command);
                        }
                    }
                }
            }
        }
        
        // 如果串口已连接，实际发送数据
        if let Some(actual_data) = &actual_sent_data {
            if let Some(port_arc) = port {
                if let Ok(mut port_guard) = port_arc.lock() {
                    port_guard.write(actual_data.as_bytes()).map_err(|e| format!("{}", e))?;
                }
            }
        }
        
        Ok(actual_sent_data)
    }
    
    // 检查是否是 WiFi 模块发送的控制消息
    pub fn is_wifi_module_control_message(&self, message: &str) -> bool {
        // 检查是否是 JSON 格式的控制消息
        message.starts_with('{') && message.contains("topic") && message.contains("payload")
    }
    
    // 从消息中提取真正的数据内容
    pub fn extract_actual_data(&self, message: &str) -> String {
        // 这里可以根据实际的消息格式进行解析
        // 目前先返回原始消息，后续可以根据需要添加更复杂的解析逻辑
        message.to_string()
    }
    
    // 从串口数据中提取主题
    pub fn extract_topic_from_serial_data(&self, data: &str) -> Option<String> {
        // 处理类似巴法云格式的数据：cmd=2&uid=xxx&topic=xxx&msg=xxx
        if data.starts_with("cmd=") {
            let parts: Vec<&str> = data.split('&').collect();
            for part in parts {
                if part.starts_with("topic=") {
                    let topic_value = part.trim_start_matches("topic=");
                    let topic = topic_value.trim().to_string();
                    if !topic.is_empty() {
                        return Some(topic);
                    }
                }
            }
        }
        // 如果不是标准格式或没有找到主题，返回None
        None
    }
    
    // 从串口数据中提取消息内容
    pub fn extract_msg_from_serial_data(&self, data: &str) -> Option<String> {
        // 处理类似巴法云格式的数据：cmd=2&uid=xxx&topic=xxx&msg=xxx
        if data.starts_with("cmd=") {
            let parts: Vec<&str> = data.split('&').collect();
            for part in parts {
                if part.starts_with("msg=") {
                    let msg_value = part.trim_start_matches("msg=");
                    let msg = msg_value.trim().to_string();
                    if !msg.is_empty() {
                        return Some(msg);
                    }
                }
            }
        }
        // 如果不是标准格式或没有找到消息内容，返回None
        None
    }
}
