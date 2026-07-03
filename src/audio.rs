// ─── 音频模块 ───
use std::path::Path;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use rodio::{Decoder, OutputStream, Sink};

use crate::models::Track;

/// 支持的音频格式
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "aac", "m4a"];

/// 获取封面缓存目录
pub fn covers_dir() -> std::path::PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("covers")))
        .unwrap_or_else(|| std::path::PathBuf::from("covers"));
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir
}

/// 根据 track 路径生成封面缩略图的缓存文件路径
pub fn cover_cache_path(track_path: &Path) -> std::path::PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    track_path.hash(&mut hasher);
    let hash = hasher.finish();
    covers_dir().join(format!("{:016x}.jpg", hash))
}

/// 提取封面并保存为缩略图（80x80 JPEG），返回缓存路径
pub fn extract_and_cache_cover(track_path: &Path) -> Option<String> {
    // 先检查缓存
    let cache_path = cover_cache_path(track_path);
    if cache_path.exists() {
        return cache_path.to_str().map(|s| s.to_string());
    }

    // 提取原始封面
    let cover_data = extract_cover(track_path)?;

    // 缩放为 80x80 缩略图并保存为 JPEG
    if let Ok(img) = image::load_from_memory(&cover_data) {
        // 用 resize_exact 确保得到 80x80 的图片（不保持宽高比）
        let thumb = img.resize_exact(80, 80, image::imageops::FilterType::Lanczos3);
        let thumb_rgb = thumb.to_rgb8();
        let (w, h) = (thumb_rgb.width(), thumb_rgb.height());
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80);
        if encoder.encode(&thumb_rgb, w, h, image::ExtendedColorType::Rgb8).is_ok() {
            let _ = std::fs::write(&cache_path, buf.into_inner());
            return cache_path.to_str().map(|s| s.to_string());
        }
    }
    None
}

/// 从缓存路径加载封面数据
pub fn load_cover_from_cache(cover_path: &str) -> Option<Vec<u8>> {
    std::fs::read(cover_path).ok()
}

/// 读取音频文件元数据
pub fn read_meta(path: &Path) -> Option<Track> {
    let f = Probe::open(path).ok()?.guess_file_type().ok()?.read().ok()?;
    let dur = f.properties().duration().as_secs_f64();
    let tag = f.first_tag();
    let title = tag.and_then(|t| t.get_string(&ItemKey::TrackTitle)).map(|s| s.into())
        .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("未知").into());
    let artist = tag.and_then(|t| t.get_string(&ItemKey::TrackArtist)).map(|s| s.into()).unwrap_or_default();
    let album = tag.and_then(|t| t.get_string(&ItemKey::AlbumTitle)).map(|s| s.into()).unwrap_or_default();
    
    // 检查封面缓存是否存在
    let cover_path = {
        let cp = cover_cache_path(path);
        if cp.exists() {
            cp.to_str().map(|s| s.to_string())
        } else {
            None
        }
    };
    
    Some(Track { 
        path: path.into(), 
        title, 
        artist, 
        album, 
        duration: dur,
        cover_path,
        cover: None,
    })
}

/// 从音频文件提取专辑封面（原始数据）
pub fn extract_cover(path: &Path) -> Option<Vec<u8>> {
    let f = Probe::open(path).ok()?.guess_file_type().ok()?.read().ok()?;
    let tag = f.first_tag()?;
    
    let pictures = tag.pictures();
    if pictures.is_empty() {
        None
    } else {
        for pic in pictures {
            if format!("{:?}", pic.pic_type()).contains("Cover") || 
               format!("{:?}", pic.pic_type()).contains("Front") {
                return Some(pic.data().to_vec());
            }
        }
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
    #[allow(dead_code)]
    pub out: OutputStream,
    pub sink: Sink,
    pub volume: f64,
}

impl Player {
    pub fn new() -> Self {
        let (out, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        Self { out, sink, volume: 0.5 }
    }

    pub fn play(&mut self, track: &Track) {
        self.sink.stop();
        if let Ok(file) = std::fs::File::open(&track.path) {
            if let Ok(src) = Decoder::new(std::io::BufReader::new(file)) {
                self.sink.append(src);
                self.sink.set_volume(self.volume as f32);
            }
        }
    }

    pub fn toggle(&mut self, playing: bool) {
        if playing { self.sink.pause(); } else { self.sink.play(); }
    }

    pub fn stop(&mut self) {
        self.sink.stop();
    }
}

/// 格式化时间（秒 → m:ss）
pub fn fmt_time(secs: f64) -> String {
    format!("{}:{:02}", secs as u64 / 60, secs as u64 % 60)
}
