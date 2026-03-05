use std::fs;
use std::path::Path;

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
}