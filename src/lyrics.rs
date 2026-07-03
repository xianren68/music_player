// ─── 歌词解析模块 ───
// 支持 .lrc（文本）和 .krc（酷狗歌词，需 XOR 解密 + zlib 解压）

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// KRC 文件解密的 XOR Key（酷狗固定 Key，来自 krc-rs 库）
/// 正确值: [64, 71, 97, 119, 94, 50, 116, 71, 81, 54, 49, 45, 206, 210, 110, 105]
const KRC_XOR_KEY: [u8; 16] = [
    64, 71, 97, 119, 94, 50, 116, 71, 81, 54, 49, 45, 206, 210, 110, 105,
];

/// 歌词行（支持逐字时间戳）
#[derive(Debug, Clone)]
pub struct LyricLine {
    /// 行开始时间（毫秒）
    pub start: u32,
    /// 行持续时间（毫秒）
    pub duration: u32,
    /// 逐字信息（用于卡拉OK效果）
    pub words: Vec<LyricWord>,
    /// 完整文本（不含时间戳）
    pub text: String,
}

/// 逐字信息
#[derive(Debug, Clone)]
pub struct LyricWord {
    /// 字文本
    pub text: String,
    /// 相对行开始时间（毫秒）
    pub start: u32,
    /// 持续时间（毫秒）
    pub duration: u32,
}

/// 逐字高亮信息
#[derive(Debug, Clone)]
pub struct WordHighlight {
    /// 字文本
    pub text: String,
    /// 是否已唱到（高亮）
    pub highlighted: bool,
}

/// 歌词数据
#[derive(Debug, Clone)]
pub struct Lyrics {
    /// 歌词行列表
    pub lines: Vec<LyricLine>,
    /// 歌词标题（从 [ti:...] 提取）
    pub title: Option<String>,
    /// 歌手（从 [ar:...] 提取）
    pub artist: Option<String>,
}

impl Lyrics {
    pub fn empty() -> Self {
        Self { lines: Vec::new(), title: None, artist: None }
    }

    /// 根据播放进度（毫秒）获取当前歌词行索引
    pub fn find_line(&self, position_ms: u32) -> Option<usize> {
        self.lines.iter().position(|line| {
            position_ms >= line.start && position_ms < line.start + line.duration
        })
    }

    /// 根据播放进度（毫秒）和行索引，获取逐字高亮信息
    /// 用于 KRC/QRC 等支持逐字时间戳的格式
    pub fn get_word_highlights(&self, position_ms: u32, line_idx: usize) -> Option<Vec<WordHighlight>> {
        let line = self.lines.get(line_idx)?;
        let line_position = position_ms.saturating_sub(line.start);

        if line.words.is_empty() {
            return None;
        }

        let mut highlights = Vec::new();
        for word in &line.words {
            // 字已开始唱就高亮（卡拉OK效果：唱过的字用高亮色）
            let highlighted = line_position >= word.start;
            highlights.push(WordHighlight {
                text: word.text.clone(),
                highlighted,
            });
        }
        Some(highlights)
    }
}

/// 在音乐文件同级别目录下查找 Lyric 目录，然后根据音频文件名查找歌词文件
/// 歌词文件名通常是 "音频文件名-哈希.krc"
pub fn find_lyrics(music_path: &Path, title: &str) -> Option<Lyrics> {
    eprintln!("[歌词] 开始查找歌词: 音乐文件={:?}, 歌名=\"{}\"", music_path, title);

    // 获取音乐文件所在目录
    let music_dir = music_path.parent()?;
    eprintln!("[歌词] 音乐文件所在目录: {:?}", music_dir);

    // 查找 Lyric 目录（不区分大小写）
    let lyric_dir = find_lyric_dir(music_dir)?;
    eprintln!("[歌词] 找到 Lyric 目录: {:?}", lyric_dir);

    // 获取音频文件名（不含扩展名），用于匹配歌词文件
    let music_file_stem = music_path.file_stem()?;
    let music_file_stem_str = music_file_stem.to_str()?;
    eprintln!("[歌词] 音频文件名（不含扩展名）: \"{}\"", music_file_stem_str);

    // 根据音频文件名查找 .krc 或 .lrc 文件
    let file_name = find_lyric_file(&lyric_dir, music_file_stem_str, title)?;
    eprintln!("[歌词] 找到歌词文件: {:?}", file_name);

    // 解析歌词文件
    let result = parse_lyrics_file(&file_name);
    match &result {
        Some(lyrics) => eprintln!("[歌词] 解析成功！共 {} 行歌词", lyrics.lines.len()),
        None => eprintln!("[歌词] 解析失败！"),
    }
    result
}

/// 查找 Lyric 目录（支持各种大小写变体）
fn find_lyric_dir(music_dir: &Path) -> Option<PathBuf> {
    eprintln!("[歌词] 正在查找 Lyric 目录...");
    let entries = fs::read_dir(music_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name()?.to_str()?.to_lowercase();
            eprintln!("[歌词]   发现子目录: {} (匹配={})", 
                path.file_name().unwrap_or_default().to_str().unwrap_or("?"),
                dir_name == "lyric" || dir_name == "lyrics");
            if dir_name == "lyric" || dir_name == "lyrics" {
                return Some(path);
            }
        }
    }
    eprintln!("[歌词] ❌ 未找到 Lyric/Lyrics 目录！");
    None
}

/// 根据音频文件名在 Lyric 目录下查找歌词文件
/// 歌词文件名通常是 "音频文件名-哈希.krc" 或 "音频文件名.krc"
fn find_lyric_file(lyric_dir: &Path, music_file_stem: &str, title: &str) -> Option<PathBuf> {
    eprintln!("[歌词] 正在查找歌词文件，音频文件名=\"{}\", 歌名=\"{}\"", music_file_stem, title);
    
    // 先收集所有歌词文件
    let entries: Vec<_> = fs::read_dir(lyric_dir).ok()?
        .flatten()
        .filter(|e| e.path().is_file())
        .collect();
    
    let music_stem_lower = music_file_stem.to_lowercase();
    let title_lower = title.to_lowercase();

    // 策略1：精确匹配 - 文件名以音频文件名开头（最精确）
    for entry in &entries {
        let path = entry.path();
        let file_name = path.file_name()?.to_str()?.to_lowercase();
        
        if (file_name.ends_with(".krc") || file_name.ends_with(".lrc") || file_name.ends_with(".qrc")) 
            && file_name.starts_with(&music_stem_lower) {
            eprintln!("[歌词] ✅ 精确匹配成功: {:?}", path);
            return Some(path);
        }
    }
    
    // 策略2：兜底 - 用歌名模糊匹配（文件名包含歌名）
    eprintln!("[歌词] 精确匹配未找到，尝试用歌名模糊匹配...");
    for entry in &entries {
        let path = entry.path();
        let file_name = path.file_name()?.to_str()?.to_lowercase();
        
        if (file_name.ends_with(".krc") || file_name.ends_with(".lrc") || file_name.ends_with(".qrc")) 
            && file_name.contains(&title_lower) {
            eprintln!("[歌词] ✅ 模糊匹配成功: {:?}", path);
            return Some(path);
        }
    }
    
    eprintln!("[歌词] ❌ 未找到匹配的歌词文件！");
    None
}

/// 解析歌词文件（自动判断 .lrc 或 .krc 或 .qrc）
fn parse_lyrics_file(path: &Path) -> Option<Lyrics> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    
    match ext.as_str() {
        "lrc" => parse_lrc_file(path),
        "krc" => parse_krc_file(path),
        "qrc" => parse_qrc_file(path),
        _ => None,
    }
}

/// 解析 .lrc 文件（简单文本格式）
fn parse_lrc_file(path: &Path) -> Option<Lyrics> {
    let content = fs::read_to_string(path).ok()?;
    let mut lyrics = Lyrics::empty();
    
    for line in content.lines() {
        let line = line.trim();
        
        // 解析元数据
        if line.starts_with("[ti:") {
            if let Some(t) = extract_meta(line) {
                lyrics.title = Some(t);
            }
            continue;
        }
        if line.starts_with("[ar:") {
            if let Some(a) = extract_meta(line) {
                lyrics.artist = Some(a);
            }
            continue;
        }
        
        // 解析歌词行：[mm:ss.xx]歌词内容
        if let Some((time_ms, text)) = parse_lrc_line(line) {
            lyrics.lines.push(LyricLine {
                start: time_ms,
                duration: 0, // LRC 格式没有行持续时间，需要在后续计算
                words: vec![LyricWord {
                    text: text.clone(),
                    start: 0,
                    duration: 0,
                }],
                text,
            });
        }
    }
    
    // 计算每行的持续时间
    for i in 0..lyrics.lines.len() {
        if i + 1 < lyrics.lines.len() {
            lyrics.lines[i].duration = lyrics.lines[i + 1].start - lyrics.lines[i].start;
        } else {
            lyrics.lines[i].duration = 5000; // 最后一行默认5秒
        }
    }
    
    Some(lyrics)
}

/// 提取 [tag:content] 中的 content
fn extract_meta(line: &str) -> Option<String> {
    let start = line.find(':')? + 1;
    let end = line.rfind(']')?;
    if start < end {
        Some(line[start..end].trim().to_string())
    } else {
        None
    }
}

/// 解析 LRC 行的时间标签
fn parse_lrc_line(line: &str) -> Option<(u32, String)> {
    // 找到第一个 ] 的位置
    let end_bracket = line.find(']')?;
    let time_str = &line[1..end_bracket];
    
    // 解析 mm:ss.xx 或 mm:ss
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let minutes: u32 = parts[0].parse().ok()?;
    let seconds_parts: Vec<&str> = parts[1].split('.').collect();
    let seconds: u32 = seconds_parts[0].parse().ok()?;
    let millis: u32 = if seconds_parts.len() > 1 {
        // 支持 .xx（百分秒）或 .xxx（毫秒）
        let ms_str = seconds_parts[1];
        match ms_str.len() {
            2 => ms_str.parse::<u32>().ok()? * 10, // .50 -> 500ms
            3 => ms_str.parse::<u32>().ok()?,       // .500 -> 500ms
            _ => 0,
        }
    } else {
        0
    };
    
    let time_ms = (minutes * 60 + seconds) * 1000 + millis;
    let text = line[end_bracket + 1..].trim().to_string();
    
    Some((time_ms, text))
}

/// 解析 .krc 文件（酷狗歌词格式）
/// 参考: https://github.com/CGQAQ/krc-rs
fn parse_krc_file(path: &Path) -> Option<Lyrics> {
    eprintln!("[KRC] 开始解析文件: {:?}", path);
    let buf = fs::read(path).ok()?;
    eprintln!("[KRC] 文件大小: {} bytes", buf.len());
    
    // 检查文件头
    if buf.len() < 4 {
        eprintln!("[KRC] ❌ 文件太小 (< 4 bytes)");
        return None;
    }
    if &buf[0..4] != b"krc1" {
        eprintln!("[KRC] ❌ 文件头不是 \"krc1\"，实际: {:?}", &buf[0..4]);
        return None;
    }
    eprintln!("[KRC] ✅ 文件头正确 (krc1)");
    
    // 去掉头部4字节
    let data = &buf[4..];
    
    // XOR 解密（使用正确的 Key，循环使用16字节）
    let mut decrypted: Vec<u8> = Vec::with_capacity(data.len());
    for (i, &byte) in data.iter().enumerate() {
        decrypted.push(byte ^ KRC_XOR_KEY[i % 16]);
    }
    eprintln!("[KRC] XOR 解密完成，{} bytes", decrypted.len());
    
    // 检查解密后的数据头部
    if decrypted.len() >= 2 {
        eprintln!("[KRC] 解密数据头部: {:02x} {:02x}", decrypted[0], decrypted[1]);
    }
    
    // 使用 ZlibDecoder 解压（参考 krc-rs 的方法）
    let mut decoder = flate2::read::ZlibDecoder::new(std::io::Cursor::new(decrypted));
    let mut content = String::new();
    match decoder.read_to_string(&mut content) {
        Ok(_) => {
            eprintln!("[KRC] ✅ zlib 解压成功！文本长度: {} 字符", content.len());
            // 安全地获取前200个字符（按字符数而不是字节数）
            let byte_pos = content.char_indices().nth(200).map(|(i, _)| i).unwrap_or(content.len());
            eprintln!("[KRC] 前200字符: {:?}", &content[..byte_pos]);
            
            // 去掉 UTF-8 BOM（如果存在）
            let content = content.trim_start_matches('\u{FEFF}');
            if content.len() < content.len() {
                eprintln!("[KRC] 检测到 UTF-8 BOM，已去掉");
            }
            
            // 解析 KRC 文本内容
            parse_krc_content(content)
        }
        Err(e) => {
            eprintln!("[KRC] ❌ zlib 解压失败: {}", e);
            None
        }
    }
}

/// 解析 .qrc 文件（QQ音乐歌词格式）
/// QRC 格式是文本格式，类似于 KRC 但标签格式不同
/// 标签格式: [time]歌词内容 或 [time,time,...]歌词内容
fn parse_qrc_file(path: &Path) -> Option<Lyrics> {
    eprintln!("[QRC] 开始解析文件: {:?}", path);
    let content = fs::read_to_string(path).ok()?;
    eprintln!("[QRC] 文件读取成功，文本长度: {} 字符", content.len());
    
    let mut lyrics = Lyrics::empty();
    let mut lines: Vec<LyricLine> = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        
        // 解析元数据
        if line.starts_with("[ti:") {
            lyrics.title = extract_meta(line);
            continue;
        }
        if line.starts_with("[ar:") {
            lyrics.artist = extract_meta(line);
            continue;
        }
        
        // 解析歌词行：QRC 格式可能是 [mm:ss.xx]歌词 或 [mm:ss.xx][mm:ss.xx]歌词
        if let Some(lyric_line) = parse_qrc_line(line) {
            lines.push(lyric_line);
        }
    }
    
    lyrics.lines = lines;
    Some(lyrics)
}

/// 解析 QRC 歌词行
/// QRC 格式: [start]text 或 [start,duration]text 或带逐字标签 [start]<word_info>text
fn parse_qrc_line(line: &str) -> Option<LyricLine> {
    // 查找第一个 ] 的位置
    let bracket_end = line.find(']')?;
    let time_str = &line[1..bracket_end];
    
    // 解析时间（可能是 mm:ss.xx 或毫秒）
    let start = parse_time_to_ms(time_str)?;
    
    // 暂时用默认值，后续可以计算
    let duration = 5000;
    
    let text_part = &line[bracket_end + 1..];
    let text = text_part.to_string();
    
    Some(LyricLine {
        start,
        duration,
        words: vec![],  // QRC 的逐字信息暂不支持
        text,
    })
}

/// 解析时间字符串为毫秒
/// 支持格式: "mm:ss.xx" 或 "毫秒数字"
fn parse_time_to_ms(time_str: &str) -> Option<u32> {
    // 尝试解析为毫秒数字
    if let Ok(ms) = time_str.parse::<u32>() {
        return Some(ms);
    }
    
    // 尝试解析为 mm:ss.xx
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let minutes: u32 = parts[0].parse().ok()?;
    let seconds_parts: Vec<&str> = parts[1].split('.').collect();
    let seconds: u32 = seconds_parts[0].parse().ok()?;
    let millis: u32 = if seconds_parts.len() > 1 {
        let ms_str = seconds_parts[1];
        match ms_str.len() {
            2 => ms_str.parse::<u32>().ok()? * 10,
            3 => ms_str.parse::<u32>().ok()?,
            _ => 0,
        }
    } else {
        0
    };
    
    Some((minutes * 60 + seconds) * 1000 + millis)
}

/// 使用 flate2 进行解压（保留原函数用于兼容）
#[allow(dead_code)]
fn decompress_zlib(input: &[u8]) -> Result<Vec<u8>, String> {
    // 使用 flate2 库进行 zlib 解压
    use std::io::Read;
    
    let mut decoder = flate2::read::ZlibDecoder::new(input);
    let mut output = Vec::new();
    
    match decoder.read_to_end(&mut output) {
        Ok(_) => Ok(output),
        Err(e) => Err(e.to_string()),
    }
}

/// 解析 KRC 文本内容
fn parse_krc_content(content: &str) -> Option<Lyrics> {
    let mut lyrics = Lyrics::empty();
    let mut lines: Vec<LyricLine> = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        
        // 解析元数据
        if line.starts_with("[ti:") {
            lyrics.title = extract_meta(line);
            continue;
        }
        if line.starts_with("[ar:") {
            lyrics.artist = extract_meta(line);
            continue;
        }
        if line.starts_with("[offset:") {
            // offset 标签，暂时忽略
            continue;
        }
        
        // 解析歌词行：[1234,456]<0,100,0>你<100,200,0>好
        if let Some(lyric_line) = parse_krc_line(line) {
            lines.push(lyric_line);
        }
    }
    
    lyrics.lines = lines;
    Some(lyrics)
}

/// 解析 KRC 歌词行
fn parse_krc_line(line: &str) -> Option<LyricLine> {
    // 格式：[start,duration]
    let bracket_end = line.find(']')?;
    let time_str = &line[1..bracket_end];
    
    let time_parts: Vec<&str> = time_str.split(',').collect();
    if time_parts.len() != 2 {
        return None;
    }
    
    let start: u32 = time_parts[0].parse().ok()?;
    let duration: u32 = time_parts[1].parse().ok()?;
    
    // 解析逐字信息：<0,100,0>你<100,200,0>好
    let text_part = &line[bracket_end + 1..];
    let (words, full_text) = parse_krc_words(text_part);
    
    Some(LyricLine {
        start,
        duration,
        words,
        text: full_text,
    })
}

/// 解析 KRC 逐字标签
fn parse_krc_words(text: &str) -> (Vec<LyricWord>, String) {
    let mut words = Vec::new();
    let mut full_text = String::new();
    
    // 简单的正则匹配：<start,duration,0>text
    let mut remaining = text;
    
    while !remaining.is_empty() {
        // 查找 < 开头
        if let Some(start_idx) = remaining.find('<') {
            let after_lt = &remaining[start_idx + 1..];
            if let Some(end_idx) = after_lt.find('>') {
                let tag_content = &after_lt[..end_idx];
                let tag_parts: Vec<&str> = tag_content.split(',').collect();
                
                if tag_parts.len() >= 2 {
                    let word_start: u32 = tag_parts[0].parse().unwrap_or(0);
                    let word_duration: u32 = tag_parts[1].parse().unwrap_or(0);
                    
                    // 提取文本（直到下一个 < 或结束）
                    let text_start = start_idx + end_idx + 2; // 跳过 <tag>
                    let text_end = remaining[text_start..].find('<').unwrap_or(remaining[text_start..].len());
                    let word_text = &remaining[text_start..text_start + text_end];
                    
                    full_text.push_str(word_text);
                    words.push(LyricWord {
                        text: word_text.to_string(),
                        start: word_start,
                        duration: word_duration,
                    });
                    
                    remaining = &remaining[text_start + text_end..];
                    continue;
                }
            }
        }
        // 如果没有更多标签，退出
        break;
    }
    
    (words, full_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_lrc_line() {
        let result = parse_lrc_line("[00:10.50]你好世界");
        assert!(result.is_some());
        let (time, text) = result.unwrap();
        assert_eq!(time, 10500);
        assert_eq!(text, "你好世界");
    }
    
    #[test]
    fn test_find_line() {
        let mut lyrics = Lyrics::empty();
        lyrics.lines.push(LyricLine {
            start: 0,
            duration: 5000,
            words: vec![],
            text: "第一行".to_string(),
        });
        lyrics.lines.push(LyricLine {
            start: 5000,
            duration: 3000,
            words: vec![],
            text: "第二行".to_string(),
        });
        
        assert_eq!(lyrics.find_line(1000), Some(0));
        assert_eq!(lyrics.find_line(6000), Some(1));
        assert_eq!(lyrics.find_line(10000), None);
    }
}
