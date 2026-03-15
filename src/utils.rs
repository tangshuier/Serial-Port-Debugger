// 显示模式
#[derive(PartialEq)]
pub enum DisplayMode {
    UTF8,
    Hex,
    Binary,
}

// 导入必要的库
use encoding_rs;

// 智能分段解码，处理混合编码的情况
pub fn smart_chunk_decode(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;
    let len = bytes.len();
    
    while i < len {
        // 1. 检查是否是UTF-8字符
        let utf8_len = try_utf8(&bytes[i..]);
        if utf8_len > 0 {
            let utf8_str = std::str::from_utf8(&bytes[i..i+utf8_len]).unwrap();
            result.push_str(utf8_str);
            i += utf8_len;
        }
        // 2. 检查是否是2字节GBK序列
        else if i + 1 < len {
            // 检查是否是有效的GBK序列
            let (gbk_char, is_gbk) = get_gbk_char(&bytes[i..i+2]);
            if is_gbk {
                result.push_str(&gbk_char);
                i += 2;
            }
            // 3. 检查是否是ASCII字符
            else if bytes[i] <= 0x7F {
                result.push(bytes[i] as char);
                i += 1;
            }
            // 4. 尝试Windows-1252解码
            else {
                let (win1252_text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes[i..i+1]);
                result.push_str(&win1252_text);
                i += 1;
            }
        }
        // 5. 只有一个字节
        else {
            if bytes[i] <= 0x7F {
                result.push(bytes[i] as char);
            } else {
                let (win1252_text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes[i..i+1]);
                result.push_str(&win1252_text);
            }
            i += 1;
        }
    }
    
    result
}

// 智能GBK解码，尝试处理部分GBK错误的情况
pub fn smart_gbk_decode(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;
    let len = bytes.len();
    
    while i < len {
        if i + 1 < len {
            // 尝试GBK解码
            let (gbk_text, _, _) = encoding_rs::GBK.decode(&bytes[i..i+2]);
            result.push_str(&gbk_text);
            i += 2;
        } else {
            // 单独处理最后一个字节
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    
    result
}

// 检查数据中是否包含可能的GBK序列
pub fn has_potential_gbk(bytes: &[u8]) -> bool {
    let mut i = 0;
    let len = bytes.len();
    
    while i < len {
        if i + 1 < len {
            // GBK序列的特征：第一个字节在0x81-0xFE之间，第二个字节在0x40-0x7E或0x80-0xFE之间
            let first = bytes[i];
            let second = bytes[i+1];
            if (first >= 0x81 && first <= 0xFE) && 
               ((second >= 0x40 && second <= 0x7E) || (second >= 0x80 && second <= 0xFE)) {
                return true;
            }
        }
        i += 1;
    }
    
    false
}

// 检查字符串中是否包含有效中文字符
pub fn has_valid_chinese(text: &str) -> bool {
    // 检查是否包含中文字符（Unicode范围：0x4E00-0x9FFF）
    text.chars().any(|c| (c as u32) >= 0x4E00 && (c as u32) <= 0x9FFF)
}

// 获取GBK字符，如果是有效的GBK序列返回字符和true，否则返回空字符串和false
pub fn get_gbk_char(bytes: &[u8]) -> (String, bool) {
    if bytes.len() < 2 {
        return (String::new(), false);
    }
    
    let first = bytes[0];
    let second = bytes[1];
    
    // 检查是否是有效的GBK序列
    if (first >= 0x81 && first <= 0xFE) && 
       ((second >= 0x40 && second <= 0x7E) || (second >= 0x80 && second <= 0xFE)) {
        let (gbk_text, _, gbk_errors) = encoding_rs::GBK.decode(bytes);
        if !gbk_errors {
            return (gbk_text.to_string(), true);
        }
    }
    
    (String::new(), false)
}

// 尝试识别UTF-8字符，返回字符长度（0表示不是UTF-8）
pub fn try_utf8(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }
    
    let first_byte = bytes[0];
    
    if first_byte <= 0x7F {
        // 1字节UTF-8
        return 1;
    } else if first_byte >= 0xC0 && first_byte <= 0xDF && bytes.len() >= 2 {
        // 2字节UTF-8
        if (bytes[1] & 0xC0) == 0x80 {
            return 2;
        }
    } else if first_byte >= 0xE0 && first_byte <= 0xEF && bytes.len() >= 3 {
        // 3字节UTF-8
        if (bytes[1] & 0xC0) == 0x80 && (bytes[2] & 0xC0) == 0x80 {
            return 3;
        }
    } else if first_byte >= 0xF0 && first_byte <= 0xF7 && bytes.len() >= 4 {
        // 4字节UTF-8
        if (bytes[1] & 0xC0) == 0x80 && (bytes[2] & 0xC0) == 0x80 && (bytes[3] & 0xC0) == 0x80 {
            return 4;
        }
    }
    
    0
}

// 根据选择的编码解析数据
pub fn try_decode(bytes: &[u8], receive_encoding: &str) -> String {
    match receive_encoding {
        "自动识别" => {
            // 1. 首先检查数据中是否包含可能的GBK序列
            let has_potential_gbk = has_potential_gbk(bytes);
            
            // 2. 尝试整体UTF-8解码
            if let Ok(text) = std::str::from_utf8(bytes) {
                return text.to_string();
            }
            
            // 3. 尝试整体GBK解码，特别是当数据中可能包含GBK序列时
            let (gbk_text, _, gbk_errors) = encoding_rs::GBK.decode(bytes);
            if !gbk_errors {
                return gbk_text.to_string();
            }
            
            // 4. 如果整体GBK解码失败但数据中可能包含GBK序列，尝试更智能的GBK解码
            if has_potential_gbk {
                let gbk_result = smart_gbk_decode(bytes);
                // 检查GBK解码结果是否包含有效中文字符
                if has_valid_chinese(&gbk_result) {
                    return gbk_result;
                }
            }
            
            // 5. 尝试整体Windows-1252解码（常见的西方编码）
            let (win1252_text, _, win1252_errors) = encoding_rs::WINDOWS_1252.decode(bytes);
            if !win1252_errors {
                return win1252_text.to_string();
            }
            
            // 6. 如果所有整体解码都失败，尝试逐段智能解码
            smart_chunk_decode(bytes)
        }
        "UTF-8" => {
            if let Ok(text) = std::str::from_utf8(bytes) {
                text.to_string()
            } else {
                bytes.iter().map(|b| format!("{:02X} ", b)).collect()
            }
        }
        "GB2312" => {
            let (text, _, _) = encoding_rs::GBK.decode(bytes);
            text.to_string()
        }
        "Big5" => {
            let (text, _, _) = encoding_rs::BIG5.decode(bytes);
            text.to_string()
        }
        "EUC-JP" => {
            let (text, _, _) = encoding_rs::EUC_JP.decode(bytes);
            text.to_string()
        }
        "Shift_JIS" => {
            let (text, _, _) = encoding_rs::SHIFT_JIS.decode(bytes);
            text.to_string()
        }
        "KOI8-R" => {
            let (text, _, _) = encoding_rs::KOI8_R.decode(bytes);
            text.to_string()
        }
        "Windows-1251" => {
            let (text, _, _) = encoding_rs::WINDOWS_1251.decode(bytes);
            text.to_string()
        }
        "Windows-1252" => {
            let (text, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            text.to_string()
        }
        "UTF-16 LE" => {
            if bytes.len() >= 2 {
                let text = encoding_rs::UTF_16LE.decode(bytes).0.to_string();
                text
            } else {
                bytes.iter().map(|b| format!("{:02X} ", b)).collect()
            }
        }
        "UTF-16 BE" => {
            if bytes.len() >= 2 {
                let text = encoding_rs::UTF_16BE.decode(bytes).0.to_string();
                text
            } else {
                bytes.iter().map(|b| format!("{:02X} ", b)).collect()
            }
        }
        "Latin1" | "ASCII" => {
            // Latin1 和 ASCII 编码：每个字节直接对应一个 Unicode 字符
            bytes.iter().map(|&b| b as char).collect()
        }
        _ => {
            if let Ok(text) = std::str::from_utf8(bytes) {
                text.to_string()
            } else {
                bytes.iter().map(|b| format!("{:02X} ", b)).collect()
            }
        }
    }
}


