use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sha1::Digest;

// GitHub 仓库信息
const REPO_OWNER: &str = "tangshuier";
const REPO_NAME: &str = "Serial-Port-Debugger";
const TARGET_FILE: &str = "target/release/串口调试器.exe";
const CACHE_FILE: &str = ".update_cache.json";
const CACHE_DURATION: Duration = Duration::from_secs(3600); // 缓存有效期 1 小时

// 缓存结构体
#[derive(serde::Serialize, serde::Deserialize)]
struct Cache {
    remote_sha: String,
    timestamp: u64,
}

// 计算文件的 SHA 哈希值
pub fn calculate_file_sha(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut hasher = sha1::Sha1::new();
    let mut buffer = [0; 8192];
    
    while let Ok(bytes_read) = file.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    let sha = bytes_to_hex(&result[..]);
    Ok(sha)
}

// 计算远程文件的 SHA 哈希值
pub fn calculate_remote_sha() -> Result<String, Box<dyn std::error::Error>> {
    let download_url = format!(
        "https://raw.githubusercontent.com/{}/{}/master/{}",
        REPO_OWNER, REPO_NAME, TARGET_FILE
    );
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Serial-Monitor/1.0")
        .timeout(Duration::from_secs(60))
        .build()?;
    
    let mut response = client.get(&download_url).send()?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP 请求失败: {:?}", response.status()).into());
    }
    
    let mut hasher = sha1::Sha1::new();
    let mut buffer = [0; 8192];
    
    while let Ok(bytes_read) = response.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    let sha = bytes_to_hex(&result[..]);
    Ok(sha)
}

// 从缓存获取远程文件的 SHA
pub fn get_cached_remote_sha() -> Result<String, Box<dyn std::error::Error>> {
    let cache_path = Path::new(CACHE_FILE);
    if !cache_path.exists() {
        return Err("缓存文件不存在".into());
    }
    
    let mut file = File::open(cache_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    let cache: Cache = serde_json::from_str(&content)?;
    
    // 检查缓存是否过期
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if current_time - cache.timestamp > CACHE_DURATION.as_secs() {
        return Err("缓存已过期".into());
    }
    
    Ok(cache.remote_sha)
}

// 更新缓存
pub fn update_cache(remote_sha: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cache = Cache {
        remote_sha: remote_sha.to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    };
    
    let content = serde_json::to_string(&cache)?;
    
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(CACHE_FILE)?;
    
    file.write_all(content.as_bytes())?;
    Ok(())
}

// 检查是否有更新
pub fn check_for_updates() -> Result<bool, Box<dyn std::error::Error>> {
    // 获取本地可执行文件的路径
    let local_path = std::env::current_exe()?;
    println!("本地可执行文件路径: {:?}", local_path);
    
    // 计算本地文件的 SHA
    let local_sha = calculate_file_sha(&local_path)?;
    println!("本地文件 SHA: {}", local_sha);
    
    // 尝试从缓存获取远程文件的 SHA
    let remote_sha = match get_cached_remote_sha() {
        Ok(cached_sha) => {
            println!("使用缓存的远程文件 SHA: {}", cached_sha);
            cached_sha
        },
        Err(_) => {
            // 缓存无效或不存在，下载远程文件并计算其 SHA 值
            println!("缓存无效或不存在，计算远程文件 SHA...");
            let sha = calculate_remote_sha()?;
            println!("远程文件内容 SHA: {}", sha);
            // 更新缓存
            if let Err(e) = update_cache(&sha) {
                println!("更新缓存失败: {:?}", e);
            }
            sha
        }
    };
    
    // 比较本地文件和远程文件的 SHA
    let update_available = local_sha != remote_sha;
    println!("是否有更新: {}", update_available);
    Ok(update_available)
}

// 下载更新（带进度回调）
pub fn download_update_with_progress<F>(progress_callback: F) -> Result<(), Box<dyn std::error::Error>> 
where
    F: Fn(f32) + Send + Sync + 'static,
{
    let download_url = format!(
        "https://raw.githubusercontent.com/{}/{}/master/{}",
        REPO_OWNER, REPO_NAME, TARGET_FILE
    );
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Serial-Monitor/1.0")
        .timeout(Duration::from_secs(120))
        .build()?;
    
    let response = client.get(&download_url).send()?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP 请求失败: {:?}", response.status()).into());
    }
    
    // 获取文件大小
    let total_size = response.content_length().unwrap_or(0);
    
    // 获取本地可执行文件的路径
    let local_path = std::env::current_exe()?;
    let backup_path = local_path.with_extension(format!("old_{}.exe", chrono::Local::now().format("%Y%m%d%H%M%S")));
    
    // 备份旧文件
    if local_path.exists() {
        std::fs::rename(&local_path, &backup_path)?;
        println!("已备份旧文件到: {:?}", backup_path);
    }
    
    // 下载新文件
    let mut dest_file = File::create(&local_path)?;
    let mut content = response;
    let mut downloaded = 0;
    let mut buffer = [0; 8192];
    
    while let Ok(bytes_read) = content.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        dest_file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read;
        
        // 计算并回调进度
        if total_size > 0 {
            let progress = downloaded as f32 / total_size as f32;
            progress_callback(progress);
        }
    }
    
    println!("文件下载成功，大小: {} 字节", downloaded);
    
    // 删除旧版备份文件
    if backup_path.exists() {
        if let Err(e) = std::fs::remove_file(&backup_path) {
            println!("删除旧版备份文件失败: {:?}", e);
        } else {
            println!("已删除旧版备份文件: {:?}", backup_path);
        }
    }
    
    Ok(())
}

// 下载更新（兼容旧接口）
pub fn download_update() -> Result<(), Box<dyn std::error::Error>> {
    download_update_with_progress(|_| {})?;
    Ok(())
}

// 将字节数组转换为十六进制字符串
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
