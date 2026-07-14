// ─── 音频模块 ───
use std::path::Path;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::ItemKey;

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

/// 清理旧版本的封面缓存（不带 v2_ 前缀的低分辨率文件）
pub fn cleanup_old_covers() {
    let dir = covers_dir();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                // 删除不带 v2_ 前缀的旧缓存文件（包括 jpg 和 png）
                if !name.starts_with("v2_") && (name.ends_with(".jpg") || name.ends_with(".png")) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }
}

/// 根据 track 路径生成封面缩略图的缓存文件路径
/// v2_ 前缀：高分辨率版本（400x400 PNG），与旧版 80x80 JPEG 区分
pub fn cover_cache_path(track_path: &Path) -> std::path::PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    track_path.hash(&mut hasher);
    let hash = hasher.finish();
    covers_dir().join(format!("v2_{:016x}.png", hash))
}

/// 提取封面并保存为缩略图（400x400 PNG），返回缓存路径
/// 使用 PNG 格式避免 JPEG YCbCr → sRGB 颜色空间转换导致的色偏
pub fn extract_and_cache_cover(track_path: &Path) -> Option<String> {
    // 先检查缓存
    let cache_path = cover_cache_path(track_path);
    if cache_path.exists() {
        return cache_path.to_str().map(|s| s.to_string());
    }

    // 提取原始封面
    let cover_data = extract_cover(track_path)?;

    // 缩放为 400x400 缩略图并保存为 PNG（无损，保留完整颜色信息）
    if let Ok(img) = image::load_from_memory(&cover_data) {
        // resize 保持宽高比（专辑封面通常为正方形）
        let thumb = img.resize(400, 400, image::imageops::FilterType::Lanczos3);
        // 直接保存为 PNG（支持 RGBA，无损压缩，避免颜色空间转换问题）
        if thumb.save(&cache_path).is_ok() {
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

// ─── 播放器后端（按平台分实现）───
// 非 Windows：用 rodio（跨平台，支持 OGG 等全部格式）。
// Windows：优先用 WinRT MediaPlayer 播放——它的音频会话天然是 "Media" 类别，
//   系统才会把本窗口识别为"正在播放媒体的应用"，任务栏缩略图上的
//   播放/暂停/上下首传输控件才会显示出来。OGG 等 Media Foundation 不原生
//   支持的格式，兜底用 rodio 播放（此时该曲无任务栏控件，但能正常出声）。

/// rodio 后端（跨平台主用；Windows 上作为 OGG 等格式的兜底）
struct RodioPlayer {
    #[allow(dead_code)]
    out: rodio::OutputStream,
    sink: rodio::Sink,
    volume: f64,
}

impl RodioPlayer {
    fn new() -> Self {
        let (out, handle) = rodio::OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&handle).unwrap();
        Self { out, sink, volume: 0.5 }
    }
    fn play(&mut self, track: &Track) {
        self.sink.stop();
        if let Ok(file) = std::fs::File::open(&track.path) {
            if let Ok(src) = rodio::Decoder::new(std::io::BufReader::new(file)) {
                self.sink.append(src);
                self.sink.set_volume(self.volume as f32);
            }
        }
    }
    fn toggle(&mut self, paused: bool) {
        if paused { self.sink.pause(); } else { self.sink.play(); }
    }
    fn stop(&mut self) {
        self.sink.stop();
    }
    fn set_volume(&mut self, v: f64) {
        self.volume = v;
        self.sink.set_volume(v as f32);
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;

    pub struct Player {
        inner: RodioPlayer,
        #[allow(dead_code)]
        volume: f64,
    }

    impl Player {
        pub fn new() -> Self {
            Self { inner: RodioPlayer::new(), volume: 0.5 }
        }
        pub fn play(&mut self, track: &Track) {
            self.inner.play(track);
        }
        pub fn toggle(&mut self, paused: bool) {
            self.inner.toggle(paused);
        }
        pub fn stop(&mut self) {
            self.inner.stop();
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use windows::Media::Playback::*;
    use windows::Media::Core::MediaSource;
    use windows::Storage;
    use windows::core::HSTRING;

    pub struct Player {
        /// WinRT MediaPlayer（MP3/FLAC/WAV/AAC/M4A 等走这里，自带 Media 类别音频会话）
        media: Option<MediaPlayer>,
        /// rodio 兜底后端（OGG 等 MediaPlayer 不支持的格式）
        rodio: Option<RodioPlayer>,
        /// 当前正在用哪个后端播放
        using_rodio: bool,
        volume: f64,
    }

    impl Player {
        pub fn new() -> Self {
            Self { media: None, rodio: None, using_rodio: false, volume: 0.5 }
        }

        pub fn play(&mut self, track: &Track) {
            let is_ogg = track.path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("ogg"))
                .unwrap_or(false);

            // OGG（Vorbis）Media Foundation 不原生支持 → 用 rodio 兜底
            if is_ogg {
                let r = self.rodio.get_or_insert_with(RodioPlayer::new);
                r.set_volume(self.volume);
                r.play(track);
                self.using_rodio = true;
                return;
            }

            // 切换走 MediaPlayer 前，先停掉可能残留的 rodio 播放
            if let Some(r) = &mut self.rodio {
                r.stop();
            }

            // 用 WinRT MediaPlayer 播放：音频会话自动是 Media 类别，
            // 系统才会把本窗口当作"媒体应用"，任务栏缩略图控件才出现。
            let mp = match MediaPlayer::new() {
                Ok(mp) => mp,
                Err(_) => {
                    // MediaPlayer 创建失败 → 兜底 rodio
                    let r = self.rodio.get_or_insert_with(RodioPlayer::new);
                    r.set_volume(self.volume);
                    r.play(track);
                    self.using_rodio = true;
                    return;
                }
            };
            // 关闭 MediaPlayer 自带的 SMTC，避免和我们 GetForWindow 的 SMTC 冲突
            // （否则系统媒体键可能路由到 MediaPlayer 自带的 SMTC，导致上下首失效）
            if let Ok(smtc) = mp.SystemMediaTransportControls() {
                let _ = smtc.SetIsEnabled(false);
            }
            match create_media_source(&track.path) {
                Some(source) => {
                    let _ = mp.SetSource(&source);
                    let _ = mp.SetVolume(self.volume);
                    let _ = mp.Play();
                    self.media = Some(mp);
                    self.using_rodio = false;
                }
                None => {
                    // MediaPlayer 打不开（罕见，如特殊路径）→ 兜底 rodio
                    let r = self.rodio.get_or_insert_with(RodioPlayer::new);
                    r.set_volume(self.volume);
                    r.play(track);
                    self.using_rodio = true;
                }
            }
        }

        pub fn toggle(&mut self, paused: bool) {
            if self.using_rodio {
                if let Some(r) = &mut self.rodio {
                    r.toggle(paused);
                }
            } else if let Some(mp) = &self.media {
                if paused {
                    let _ = mp.Pause();
                } else {
                    let _ = mp.Play();
                }
            }
        }

        pub fn stop(&mut self) {
            if let Some(mp) = &self.media {
                let _ = mp.Pause();
            }
            if let Some(r) = &mut self.rodio {
                r.stop();
            }
        }
    }

    /// 从本地文件路径创建 MediaSource。用 StorageFile 比 file:// URI 更可靠
    /// （避免空格/特殊字符的 URL 编码问题）。
    fn create_media_source(path: &Path) -> Option<MediaSource> {
        let path_str = path.to_string_lossy().to_string();
        let op = Storage::StorageFile::GetFileFromPathAsync(&HSTRING::from(path_str)).ok()?;
        let file = op.get().ok()?;
        MediaSource::CreateFromStorageFile(&file).ok()
    }
}

pub use imp::Player;

/// 格式化时间（秒 → m:ss）
pub fn fmt_time(secs: f64) -> String {
    format!("{}:{:02}", secs as u64 / 60, secs as u64 % 60)
}
