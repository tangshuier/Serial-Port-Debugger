use serialport::{SerialPort, SerialPortType};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};

// 串口通信相关功能
pub struct SerialManager {
    pub available_ports: Vec<String>,
    pub selected_port: Option<String>,
    pub baud_rate: u32,
    pub baud_rates: Vec<u32>,
    pub data_bits: serialport::DataBits,
    pub parity: serialport::Parity,
    pub stop_bits: serialport::StopBits,
    pub is_connected: bool,
    pub port_rx: Option<mpsc::Receiver<Vec<u8>>>,
    pub port: Option<Arc<Mutex<Box<dyn SerialPort>>>>,
    // 线程控制标志
    pub running: Arc<AtomicBool>,
    // 接收线程句柄
    pub receive_thread: Option<thread::JoinHandle<()>>,
}

impl Default for SerialManager {
    fn default() -> Self {
        Self {
            available_ports: vec!(),
            selected_port: None,
            baud_rate: 115200,
            baud_rates: vec![9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600],
            data_bits: serialport::DataBits::Eight,
            parity: serialport::Parity::None,
            stop_bits: serialport::StopBits::One,
            is_connected: false,
            port_rx: None,
            port: None,
            running: Arc::new(AtomicBool::new(false)),
            receive_thread: None,
        }
    }
}

impl SerialManager {
    // 扫描可用串口
    pub fn scan_ports(&mut self) {
        self.available_ports.clear();
        let mut port_map = std::collections::HashMap::new();
        let mut first_ch340_port = None;
        
        if let Ok(ports) = serialport::available_ports() {
            for port in ports {
                // 检查是否为 CH340 设备
                let is_ch340 = match port.port_type {
                    SerialPortType::UsbPort(info) => {
                        // CH340 的 VID 和 PID
                        (info.vid == 0x1A86 && info.pid == 0x7523) || 
                        (info.vid == 0x4348 && info.pid == 0x5523)
                    }
                    _ => false,
                };
                
                let port_name = if is_ch340 {
                    format!("{}\t(CH340)", port.port_name)
                } else {
                    port.port_name
                };
                
                // 保存第一个找到的 CH340 端口
                if is_ch340 && first_ch340_port.is_none() {
                    first_ch340_port = Some(port_name.clone());
                }
                
                self.available_ports.push(port_name.clone());
                // 保存端口名称映射（去掉后缀的端口名 -> 完整端口名）
                port_map.insert(port_name.split('\t').next().unwrap().to_string(), port_name);
            }
        }
        
        // 自动选择第一个 CH340 端口（如果存在且当前没有选择端口）
        if let Some(ch340_port) = first_ch340_port {
            // 如果当前没有选择端口，或者选择的端口不是 CH340，则自动选择
            let is_current_ch340 = self.selected_port.as_ref().map(|p| p.contains("CH340")).unwrap_or(false);
            if self.selected_port.is_none() || !is_current_ch340 {
                self.selected_port = Some(ch340_port);
            }
        }
        
        // 检查并更新 selected_port，确保它与下拉框中的选项匹配
        if let Some(selected) = &self.selected_port {
            // 如果 selected 不在 available_ports 中，尝试匹配去掉后缀的端口名
            if !self.available_ports.contains(selected) {
                // 提取实际端口名（去掉可能的后缀）
                let actual_port = selected.split('\t').next().unwrap().to_string();
                // 尝试在 port_map 中查找对应的完整端口名
                if let Some(full_port_name) = port_map.get(&actual_port) {
                    self.selected_port = Some(full_port_name.clone());
                }
            }
        }
    }

    // 连接到串口
    pub fn connect(&mut self) -> Result<(), String> {
        // 先断开之前的连接
        self.disconnect();
        
        if let Some(port_str) = &self.selected_port {
            // 提取实际端口名称（去掉 CH340 标记）
            let actual_port = port_str.split('\t').next().unwrap().to_string();
            
            match serialport::new(actual_port, self.baud_rate)
                .data_bits(self.data_bits)
                .parity(self.parity)
                .stop_bits(self.stop_bits)
                .flow_control(serialport::FlowControl::None)
                .open() {
                Ok(mut port) => {
                    // 设置串口超时为10毫秒，平衡响应速度和锁竞争
                    port.set_timeout(std::time::Duration::from_millis(10)).unwrap_or_else(|e| {
                        println!("设置超时失败: {}", e);
                    });
                    
                    self.is_connected = true;
                    
                    // 创建共享的串口对象
                    let port_arc = Arc::new(Mutex::new(port));
                    self.port = Some(port_arc.clone());
                    
                    // 创建通道用于接收数据
                    let (tx, rx) = mpsc::channel();
                    self.port_rx = Some(rx);
                    
                    // 设置运行标志
                    self.running = Arc::new(AtomicBool::new(true));
                    let running_clone = self.running.clone();
                    
                    // 启动线程接收数据
                    let handle = thread::spawn(move || {
                        let mut buffer = [0; 1024];
                        while running_clone.load(Ordering::Relaxed) {
                            // 尝试获取锁，使用try_lock避免长时间阻塞
                            if let Ok(mut port_guard) = port_arc.try_lock() {
                                match port_guard.read(&mut buffer) {
                                    Ok(bytes_read) => {
                                        if bytes_read > 0 {
                                            let data = buffer[..bytes_read].to_vec();
                                            // 发送原始数据
                                            if let Err(_) = tx.send(data) {
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // 只处理非超时错误
                                        if e.kind() != std::io::ErrorKind::TimedOut {
                                            println!("串口读取错误: {}", e);
                                        }
                                    }
                                }
                            }
                            // 增加休眠时间，减少锁竞争
                            thread::sleep(Duration::from_millis(20));
                        }
                    });
                    
                    self.receive_thread = Some(handle);
                    Ok(())
                }
                Err(e) => {
                    Err(format!("连接失败: {}", e))
                }
            }
        } else {
            Err("未选择串口".to_string())
        }
    }

    // 断开串口连接
    pub fn disconnect(&mut self) {
        // 设置运行标志为 false，让接收线程退出
        self.running.store(false, Ordering::Relaxed);
        
        // 等待接收线程退出
        if let Some(handle) = self.receive_thread.take() {
            let _ = handle.join();
        }
        
        // 清理资源
        self.is_connected = false;
        self.port_rx = None;
        self.port = None;
    }

    // 发送数据
    pub fn send_data(&self, data: &[u8]) -> Result<(), std::io::Error> {
        if let Some(port_arc) = &self.port {
            // 尝试获取锁，最多尝试5次
            for i in 0..5 {
                if let Ok(mut port_guard) = port_arc.try_lock() {
                    port_guard.write(data)?;
                    return Ok(());
                }
                // 指数退避策略，增加等待时间
                let wait_time = 5 * (i + 1);
                std::thread::sleep(std::time::Duration::from_millis(wait_time as u64));
            }
            Err(std::io::Error::new(std::io::ErrorKind::Other, "无法获取串口锁"))
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "Not connected to serial port"))
        }
    }

    // 处理接收到的数据
    pub fn process_received_data(&self) -> Option<Vec<u8>> {
        if let Some(rx) = &self.port_rx {
            match rx.try_recv() {
                Ok(data) => Some(data),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}
