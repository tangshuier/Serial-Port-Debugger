use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;

// GitHub 仓库信息
const REPO_OWNER: &str = "tangshuier";
const REPO_NAME: &str = "Serial-Port-Debugger";
const TARGET_FILE: &str = "target/release/串口调试器.exe";
const UPDATE_BRANCH: &str = "release"; // 更新检测的分支
const CACHE_DURATION: Duration = Duration::from_secs(3600); // 缓存有效期 1 小时

// 获取更新分支
fn get_update_branch() -> String {
    UPDATE_BRANCH.to_string()
}

// 获取远程版本号
pub fn get_remote_version() -> Result<String, Box<dyn std::error::Error>> {
    let branch = get_update_branch();
    let cargo_toml_url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/Cargo.toml",
        REPO_OWNER, REPO_NAME, branch
    );
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Serial-Monitor/1.0")
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let response = client.get(&cargo_toml_url).send()?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP 请求失败: {:?}", response.status()).into());
    }
    
    let content = response.text()?;
    
    // 解析Cargo.toml文件，提取version字段
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("version = ") {
            let version = line.split_once('=').unwrap().1.trim().trim_matches('"');
            return Ok(version.to_string());
        }
    }
    
    Err("未找到版本号".into())
}





// 解析版本号为元组 (major, minor, patch)
fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    
    let major = parts[0].parse::<u32>().ok()?;
    let minor = parts[1].parse::<u32>().ok()?;
    let patch = parts[2].parse::<u32>().ok()?;
    Some((major, minor, patch))
}

// 比较版本号，返回 true 如果 remote_version 比 local_version 新
fn is_version_newer(local_version: &str, remote_version: &str) -> bool {
    let Some(local) = parse_version(local_version) else {
        return false;
    };
    let Some(remote) = parse_version(remote_version) else {
        return false;
    };
    
    remote > local
}

// 获取本地版本号
pub fn get_local_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// 获取仓库信息
pub fn get_repo_info() -> (String, String) {
    (REPO_OWNER.to_string(), REPO_NAME.to_string())
}

// 获取当前更新分支
pub fn get_current_branch() -> String {
    get_update_branch()
}

// 获取所有版本（分支）
pub fn get_all_versions() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/branches",
        REPO_OWNER, REPO_NAME
    );
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Serial-Monitor/1.0")
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let response = client.get(&api_url).send()?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP 请求失败: {:?}", response.status()).into());
    }
    
    let branches: serde_json::Value = response.json()?;
    let mut versions = Vec::new();
    
    if let Some(branch_array) = branches.as_array() {
        for branch in branch_array {
            if let Some(name) = branch["name"].as_str() {
                // 只包含以Serial-Port-Debugger开头的分支
                if name.starts_with("Serial-Port-Debugger") {
                    versions.push(name.to_string());
                }
            }
        }
    }
    
    Ok(versions)
}

// 检查是否有更新（仅使用版本号）
pub fn check_for_updates() -> Result<bool, Box<dyn std::error::Error>> {
    // 获取本地版本号
    let local_version = get_local_version();
    println!("本地版本: {}", local_version);
    
    // 获取远程版本号
    let remote_version = get_remote_version()?;
    println!("远程版本: {}", remote_version);
    
    // 比较版本号
    let update_available = is_version_newer(&local_version, &remote_version);
    println!("版本号检查：是否有更新: {}", update_available);
    Ok(update_available)
}

// 下载更新（带进度回调）
pub fn download_update_with_progress<F>(progress_callback: F) -> Result<(), Box<dyn std::error::Error>> 
where
    F: Fn(f32) + Send + Sync + 'static,
{
    let branch = get_update_branch();
    let download_url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        REPO_OWNER, REPO_NAME, branch, TARGET_FILE
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


