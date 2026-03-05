use std::net::TcpStream;
use std::io::{Read, Write};

// 云端通信管理器
pub struct CloudManager {
    // 云端连接状态
    pub connected: bool,
    // 云端连接
    pub tcp_stream: Option<TcpStream>,
    // 云端服务设置
    pub service: String,
    pub protocol: String,
    pub uid: String,
    pub subscribe_topics: Vec<String>,
    pub publish_topics: Vec<String>,
    pub show_debug_info: bool,
}

impl Default for CloudManager {
    fn default() -> Self {
        Self {
            connected: false,
            tcp_stream: None,
            service: "巴法云".to_string(),
            protocol: "TCP".to_string(),
            uid: "".to_string(),
            subscribe_topics: Vec::new(),
            publish_topics: Vec::new(),
            show_debug_info: true,
        }
    }
}

impl CloudManager {
    // 从配置创建实例
    pub fn from_config(
        service: &str,
        protocol: &str,
        uid: &str,
        subscribe_topics: &Vec<String>,
        publish_topics: &Vec<String>,
        show_debug_info: bool
    ) -> Self {
        Self {
            connected: false,
            tcp_stream: None,
            service: service.to_string(),
            protocol: protocol.to_string(),
            uid: uid.to_string(),
            subscribe_topics: subscribe_topics.clone(),
            publish_topics: publish_topics.clone(),
            show_debug_info,
        }
    }
    
    // 连接巴法云
    pub fn connect_to_bemfa(&mut self) -> Result<(), String> {
        if self.service != "巴法云" {
            return Err("当前仅支持巴法云连接".to_string());
        }
        
        if self.protocol != "TCP" {
            return Err("当前仅支持TCP协议连接".to_string());
        }
        
        if self.uid.is_empty() {
            return Err("请输入云端私钥(UID)".to_string());
        }
        
        // 连接到巴法云服务器
        let stream = match TcpStream::connect("bemfa.com:8344") {
            Ok(stream) => stream,
            Err(e) => return Err(format!("连接失败: {}", e)),
        };
        
        // 设置非阻塞模式
        stream.set_nonblocking(true).map_err(|e| format!("设置非阻塞失败: {}", e))?;
        
        self.tcp_stream = Some(stream);
        self.connected = true;
        
        // 订阅主题
        self.subscribe_to_topic()?;
        
        Ok(())
    }
    
    // 订阅主题
    pub fn subscribe_to_topic(&mut self) -> Result<(), String> {
        if let Some(stream) = &mut self.tcp_stream {
            for topic in &self.subscribe_topics {
                if !topic.is_empty() {
                    // 构建订阅指令
                    let subscribe_cmd = format!("cmd=1&uid={}&topic={}\r\n", self.uid, topic);
                    stream.write_all(subscribe_cmd.as_bytes()).map_err(|e| format!("发送订阅指令失败: {}", e))?;
                }
            }
            Ok(())
        } else {
            Err("未连接到云端服务器".to_string())
        }
    }
    
    // 发布数据
    pub fn publish_data(&mut self, data: &str) -> Result<(), String> {
        if let Some(stream) = &mut self.tcp_stream {
            for topic in &self.publish_topics {
                if !topic.is_empty() {
                    // 构建发布指令
                    let publish_cmd = format!("cmd=2&uid={}&topic={}&msg={}\r\n", self.uid, topic, data);
                    stream.write_all(publish_cmd.as_bytes()).map_err(|e| format!("发送发布指令失败: {}", e))?;
                }
            }
            Ok(())
        } else {
            Err("未连接到云端服务器".to_string())
        }
    }
    
    // 发送心跳
    pub fn send_heartbeat(&mut self) -> Result<(), String> {
        if let Some(stream) = &mut self.tcp_stream {
            // 发送心跳指令
            let heartbeat_cmd = "ping\r\n";
            stream.write_all(heartbeat_cmd.as_bytes()).map_err(|e| format!("发送心跳失败: {}", e))?;
            Ok(())
        } else {
            Err("未连接到云端服务器".to_string())
        }
    }
    
    // 处理云端接收到的数据
    pub fn process_received_data(&mut self) -> Option<String> {
        if let Some(stream) = &mut self.tcp_stream {
            let mut buffer = [0; 1024];
            match stream.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    let response = String::from_utf8_lossy(&buffer[..n]).to_string();
                    Some(response)
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 非阻塞模式下的正常错误，忽略
                    None
                }
                Err(e) => {
                    self.connected = false;
                    self.tcp_stream = None;
                    Some(format!("云端接收错误: {}\n", e))
                }
                _ => {
                    None
                }
            }
        } else {
            None
        }
    }
    
    // 从云端响应中提取topic和消息内容
    pub fn extract_message_from_response(&self, response: &str) -> Option<(String, String)> {
        // 处理巴法云格式：cmd=2&uid=xxx&topic=xxx&msg=xxx
        if response.starts_with("cmd=2&") {
            let parts: Vec<&str> = response.split('&').collect();
            let mut topic = String::new();
            let mut msg = String::new();
            
            for part in parts {
                if part.starts_with("topic=") {
                    let topic_value = part.trim_start_matches("topic=");
                    topic = topic_value.trim().to_string();
                } else if part.starts_with("msg=") {
                    let msg_value = part.trim_start_matches("msg=");
                    msg = msg_value.trim().to_string();
                }
            }
            
            if !topic.is_empty() && !msg.is_empty() {
                return Some((topic, msg));
            }
        }
        // 如果不是标准的消息格式或缺少必要字段，返回None
        None
    }
    
    // 断开巴法云连接
    pub fn disconnect(&mut self) {
        self.tcp_stream = None;
        self.connected = false;
    }
}
