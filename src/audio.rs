// ─── 音频模块 ───
use std::path::Path;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use rodio::{Decoder, OutputStream, Sink};

use crate::models::Track;

/// 支持的音频格式
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "aac", "m4a"];

/// 读取音频文件元数据
pub fn read_meta(path: &Path) -> Option<Track> {
    let f = Probe::open(path).ok()?.guess_file_type().ok()?.read().ok()?;
    let dur = f.properties().duration().as_secs_f64();
    let tag = f.first_tag();
    let title = tag.and_then(|t| t.get_string(&ItemKey::TrackTitle)).map(|s| s.into())
        .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("未知").into());
    let artist = tag.and_then(|t| t.get_string(&ItemKey::TrackArtist)).map(|s| s.into()).unwrap_or_default();
    let album = tag.and_then(|t| t.get_string(&ItemKey::AlbumTitle)).map(|s| s.into()).unwrap_or_default();
    
    Some(Track { 
        path: path.into(), 
        title, 
        artist, 
        album, 
        duration: dur,
        cover: None,
    })
}

/// 从音频文件提取专辑封面
pub fn extract_cover(path: &Path) -> Option<Vec<u8>> {
    let f = Probe::open(path).ok()?.guess_file_type().ok()?.read().ok()?;
    let tag = f.first_tag()?;
    
    // 获取所有图片，优先使用封面
    let pictures = tag.pictures();
    if pictures.is_empty() {
        None
    } else {
        // 尝试找到封面图片
        for pic in pictures {
            // 检查是否为封面（Front Cover）
            if format!("{:?}", pic.pic_type()).contains("Cover") || 
               format!("{:?}", pic.pic_type()).contains("Front") {
                return Some(pic.data().to_vec());
            }
        }
        // 如果没有封面，返回第一张图片
        pictures.first().map(|p| p.data().to_vec())
    }
}

/// 扫描文件夹，提取所有音频文件
pub fn scan_dir(dir: &Path) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if let Ok(e) = std::fs::read_dir(&d) {
            for entry in e.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                        if let Some(t) = read_meta(&p) {
                            tracks.push(t);
                        }
                    }
                }
            }
        }
    }
    tracks.sort_by(|a, b| a.title.cmp(&b.title));
    tracks
}

/// 音频播放器
pub struct Player {
    /// 输出流（必须保持存活，否则 Sink 无法工作）
    #[allow(dead_code)]
    pub out: OutputStream,
    pub sink: Sink,
    pub volume: f64,
}

impl Player {
    /// 创建新的播放器实例
    pub fn new() -> Self {
        let (out, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        Self { out, sink, volume: 0.5 }
    }

    /// 播放指定曲目
    pub fn play(&mut self, track: &Track) {
        self.sink.stop();
        if let Ok(file) = std::fs::File::open(&track.path) {
            if let Ok(src) = Decoder::new(std::io::BufReader::new(file)) {
                self.sink.append(src);
                self.sink.set_volume(self.volume as f32);
            }
        }
    }

    /// 暂停/继续播放
    pub fn toggle(&mut self, playing: bool) {
        if playing { self.sink.pause(); } else { self.sink.play(); }
    }

    /// 停止播放
    pub fn stop(&mut self) {
        self.sink.stop();
    }
}

/// 格式化时间（秒 → m:ss）
pub fn fmt_time(secs: f64) -> String {
    format!("{}:{:02}", secs as u64 / 60, secs as u64 % 60)
}
