use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;

// 仓库信息
const REPO_OWNER: &str = "tangshuier";
const REPO_NAME: &str = "serial-port-debugger";
const TARGET_FILE: &str = "target/release/串口调试器.exe";

// Gitee API 端点
const GITEE_RAW_URL: &str = "https://gitee.com/{owner}/{repo}/raw/{branch}/{file}";
const GITEE_RELEASES_URL: &str = "https://gitee.com/api/v5/repos/{owner}/{repo}/releases";

/// 获取所有版本（标签）
/// 
/// 从Gitee仓库获取所有符合语义化版本规范的标签
/// 只包含以v开头的标准版本号标签，格式为vX.Y.Z
pub fn get_all_versions() -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let api_url = format!(
        "https://gitee.com/api/v5/repos/{}/{}/tags",
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
    
    let tags: serde_json::Value = response.json()?;
    let mut versions = Vec::new();
    
    if let Some(tag_array) = tags.as_array() {
        for tag in tag_array {
            if let Some(name) = tag["name"].as_str() {
                // 只包含以v开头的标准版本号标签，遵循语义化版本规定：v主版本号.次版本号.修订号
                if name.starts_with("v") {
                    // 验证版本号格式：vX.Y.Z
                    let version_part = &name[1..];
                    let parts: Vec<&str> = version_part.split('.').collect();
                    if parts.len() == 3 {
                        // 验证每个部分都是数字
                        let is_valid = parts.iter().all(|part| part.chars().all(|c| c.is_digit(10)));
                        if is_valid {
                            // 获取标签描述
                            let description = tag["message"].as_str().unwrap_or("").to_string();
                            versions.push((name.to_string(), description));
                        }
                    }
                }
            }
        }
    }
    
    Ok(versions)
}

/// 获取指定标签的发布信息
fn get_release_by_tag(tag: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let releases_url = GITEE_RELEASES_URL
        .replace("{owner}", REPO_OWNER)
        .replace("{repo}", REPO_NAME);
    
    // 创建带有认证的客户端
    let mut headers = reqwest::header::HeaderMap::new();
    // 添加Gitee个人访问令牌认证
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str("token 328fcabd60277c9477b2320e3a9873f6").unwrap()
    );
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Serial-Monitor/1.0")
        .timeout(Duration::from_secs(30))
        .default_headers(headers)
        .build()?;
    
    let response = client.get(&releases_url).send()?;
    
    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().unwrap_or_else(|_| "无法获取错误内容".to_string());
        return Err(format!("获取发布信息失败: {:?}, 响应: {}", status, error_body).into());
    }
    
    let releases: serde_json::Value = response.json()?;
    
    // 查找对应标签的发布
    if let Some(release_array) = releases.as_array() {
        for release in release_array {
            if let Some(release_tag) = release["tag_name"].as_str() {
                // 支持多种标签格式：
                // 1. 完整格式: Serial-Port-Debugger.1.2.6
                // 2. 简单格式: v1.2.6 或 1.2.6
                // 3. 其他格式: 如用户创建的 v0.0.1
                
                // 提取版本号部分进行比较
                let tag_version = tag.split('.').skip(1).collect::<Vec<&str>>().join(".");
                let release_version = if release_tag.starts_with("v") {
                    release_tag[1..].to_string()
                } else if release_tag.starts_with("Serial-Port-Debugger.") {
                    release_tag.split('.').skip(1).collect::<Vec<&str>>().join(".")
                } else {
                    release_tag.to_string()
                };
                
                if release_tag == tag || tag_version == release_version {
                    return Ok(release.clone());
                }
            }
        }
    }
    
    Err("未找到对应标签的发布".into())
}

/// 解析版本号为元组 (major, minor, patch)
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

/// 比较版本号，返回 true 如果 remote_version 比 local_version 新
fn is_version_newer(local_version: &str, remote_version: &str) -> bool {
    let Some(local) = parse_version(local_version) else {
        return false;
    };
    let Some(remote) = parse_version(remote_version) else {
        return false;
    };
    
    remote > local
}

/// 获取本地版本号
pub fn get_local_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 检查是否有更新
/// 
/// 比较本地版本与远程最新版本
pub fn check_for_updates() -> Result<(bool, String), Box<dyn std::error::Error>> {
    // 获取本地版本号
    let local_version = get_local_version();
    
    // 获取所有版本
    let versions = get_all_versions()?;
    
    if versions.is_empty() {
        return Ok((false, "".to_string()));
    }
    
    // 找到最新的版本
    let mut latest_version = "";
    let mut latest_version_full = "";
    
    for (version_full, _) in &versions {
        // 提取版本号部分（去掉v前缀）
        if version_full.starts_with("v") {
            let version = &version_full[1..];
            if latest_version.is_empty() || is_version_newer(latest_version, version) {
                latest_version = version;
                latest_version_full = version_full;
            }
        }
    }
    
    if latest_version.is_empty() {
        return Ok((false, "".to_string()));
    }
    
    // 比较版本号
    let update_available = is_version_newer(&local_version, latest_version);
    Ok((update_available, latest_version_full.to_string()))
}

/// 下载指定版本的更新（带进度回调）
/// 
/// 尝试从Gitee Releases API下载，失败则使用原始方法
/// 支持大文件下载，包含文件备份和错误处理
pub fn download_specific_version_with_progress<F>(version: &str, progress_callback: F) -> Result<(), Box<dyn std::error::Error>> 
where
    F: Fn(f32) + Send + Sync + 'static,
{
    // 尝试通过Releases API下载
    match get_release_by_tag(version) {
        Ok(release) => {
            // 查找资产文件
            if let Some(assets) = release["assets"].as_array() {
                for asset in assets {
                    if let Some(name) = asset["name"].as_str() {
                        if name == "串口调试器.exe" {
                            if let Some(browser_download_url) = asset["browser_download_url"].as_str() {
                                // 创建带有认证的客户端
                                let mut headers = reqwest::header::HeaderMap::new();
                                // 添加Gitee个人访问令牌认证
                                headers.insert(
                                    reqwest::header::AUTHORIZATION,
                                    reqwest::header::HeaderValue::from_str("token 328fcabd60277c9477b2320e3a9873f6").unwrap()
                                );
                                
                                let client = reqwest::blocking::Client::builder()
                                    .user_agent("Serial-Monitor/1.0")
                                    .timeout(Duration::from_secs(120))
                                    .default_headers(headers)
                                    .build()?;
                                
                                let response = client.get(browser_download_url).send()?;
                                
                                let status = response.status();
                                if !status.is_success() {
                                    // 继续尝试原始方法
                                } else {
                                    // 下载成功，处理文件
                                    let total_size = response.content_length().unwrap_or(0);
                                    
                                    let local_path = std::env::current_exe()?;
                                    let backup_path = local_path.with_extension(format!("old_{}.exe", chrono::Local::now().format("%Y%m%d%H%M%S")));
                                    
                                    if local_path.exists() {
                                        std::fs::rename(&local_path, &backup_path)?;
                                    }
                                    
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
                                        
                                        if total_size > 0 {
                                            let progress = downloaded as f32 / total_size as f32;
                                            progress_callback(progress);
                                        }
                                    }
                                    
                                    // 删除旧版备份文件
                                    if backup_path.exists() {
                                        if let Err(e) = std::fs::remove_file(&backup_path) {
                                            println!("删除旧版备份文件失败: {:?}", e);
                                        }
                                    }
                                    
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
            // 继续尝试原始方法
        }
    }
    
    // 原始下载方法（备用）
    let download_url = GITEE_RAW_URL
        .replace("{owner}", REPO_OWNER)
        .replace("{repo}", REPO_NAME)
        .replace("{branch}", version)
        .replace("{file}", TARGET_FILE);
    
    // 创建带有认证的客户端
    let mut headers = reqwest::header::HeaderMap::new();
    // 添加Gitee个人访问令牌认证
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str("token 328fcabd60277c9477b2320e3a9873f6").unwrap()
    );
    
    let client = reqwest::blocking::Client::builder()
        .user_agent("Serial-Monitor/1.0")
        .timeout(Duration::from_secs(120))
        .default_headers(headers)
        .build()?;
    
    let response = client.get(&download_url).send()?;
    
    let status = response.status();
    if !status.is_success() {
        // 获取错误响应内容
        let error_body = response.text().unwrap_or_else(|_| "无法获取错误内容".to_string());
        
        // 处理大文件需要登录的情况
        if error_body.contains("large file require login") {
            return Err("大文件下载需要登录，请手动从Gitee仓库下载对应版本的文件".into());
        }
        
        return Err(format!("HTTP 请求失败: {:?}, 响应: {}", status, error_body).into());
    }
    
    // 获取文件大小
    let total_size = response.content_length().unwrap_or(0);
    
    // 获取本地可执行文件的路径
    let local_path = std::env::current_exe()?;
    let backup_path = local_path.with_extension(format!("old_{}.exe", chrono::Local::now().format("%Y%m%d%H%M%S")));
    
    // 备份旧文件
    if local_path.exists() {
        std::fs::rename(&local_path, &backup_path)?;
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
    
    // 删除旧版备份文件
    if backup_path.exists() {
        if let Err(e) = std::fs::remove_file(&backup_path) {
            println!("删除旧版备份文件失败: {:?}", e);
        }
    }
    
    Ok(())
}