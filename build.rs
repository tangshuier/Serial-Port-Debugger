use std::fs;
use std::path::Path;

// 复制目录的函数
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn main() {
    #[cfg(target_os = "windows")] {
        // 生成.ico文件的路径
        let ico_path = "target/app_icon.ico";
        
        // 只在需要时生成.ico文件
        if !Path::new(ico_path).exists() {
            // 运行命令将png转换为ico
            // 注意：这里假设系统中安装了imagemagick或其他可以转换图片的工具
            // 但为了避免依赖外部工具，我们使用rust的image crate
            let png_path = "src/串口设置.png";
            
            // 使用image crate加载png并保存为ico
            let img = image::open(png_path).expect("Failed to open PNG image");
            
            // 确保target目录存在
            if !Path::new("target").exists() {
                fs::create_dir_all("target").expect("Failed to create target directory");
            }
            
            // 保存为ico格式
            img.save(ico_path).expect("Failed to save ICO image");
        }
        
        // 使用winres设置ico图标
        let mut res = winres::WindowsResource::new();
        res.set_icon(ico_path);
        res.compile().expect("Failed to compile Windows resource");
    }
    
    // 复制固件程序文件夹到target目录及其子目录
    let target_dir = "target";
    let firmware_dirs = ["专有固件程序", "AT固件程序"];
    let subdirs = ["debug", "release"];
    
    // 复制到target根目录
    for dir in firmware_dirs.iter() {
        let src_path = Path::new(dir);
        let dst_path = Path::new(target_dir).join(dir);
        
        if src_path.exists() {
            println!("Copying {} to {}", src_path.display(), dst_path.display());
            if let Err(e) = copy_dir_all(src_path, dst_path) {
                println!("Warning: Failed to copy {}: {}", dir, e);
            }
        }
    }
    
    // 复制到debug和release目录
    for subdir in subdirs.iter() {
        for dir in firmware_dirs.iter() {
            let src_path = Path::new(dir);
            let dst_path = Path::new(target_dir).join(subdir).join(dir);
            
            if src_path.exists() {
                println!("Copying {} to {}", src_path.display(), dst_path.display());
                if let Err(e) = copy_dir_all(src_path, dst_path) {
                    println!("Warning: Failed to copy {} to {}: {}", dir, subdir, e);
                }
            }
        }
    }
}