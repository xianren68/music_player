// ─── 音频模块 ───
use std::path::Path;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::ItemKey;
// id3 的 title()/artist()/album() 是 TagLike trait 提供的方法，必须导入 trait 才能调用
use id3::TagLike;

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

/// 读取音频文件元数据。
///
/// 设计：扩展名命中白名单的文件**无论如何都加入列表**——lofty 解析整文件失败
/// 读取音频文件元数据。
///
/// 主路径用 lofty（快、格式支持广）；当 lofty 整文件解析失败
/// （文件损坏 / 编码不支持 / 某些带异常帧的 MP3 等）时：
/// - 若是 MP3，用 `id3` crate 回退读取——其容错极强，能跳过 lofty 会整体
///   报错的异常帧（如部分 APIC / 自定义 TXXX），从而拿到真标题/歌手/专辑，
///   时长再用 `mp3_duration` 单独估算（只扫帧头、不解析标签，标签损坏也能算）；
/// - 其它格式或 id3 也失败，则退化为仅文件名 + 空歌手专辑 + duration=0 的
///   Track，照样进扫描结果（不再有"少了歌"的现象）。
pub fn read_meta(path: &Path) -> Track {    // 文件名兜底（读不出标签时，标题用文件名）
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未知")
        .to_string();

    // 主路径：lofty 完整解析
    let parsed = Probe::open(path)
        .ok()
        .and_then(|p| p.guess_file_type().ok())
        .and_then(|p| p.read().ok());

    if let Some(f) = parsed {
        let dur = f.properties().duration().as_secs_f64();
        let tag = f.first_tag();
        let title = tag
            .and_then(|t| t.get_string(&ItemKey::TrackTitle))
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback_title.clone());
        let artist = tag
            .and_then(|t| t.get_string(&ItemKey::TrackArtist))
            .map(|s| s.to_string())
            .unwrap_or_default();
        let album = tag
            .and_then(|t| t.get_string(&ItemKey::AlbumTitle))
            .map(|s| s.to_string())
            .unwrap_or_default();
        Track {
            path: path.into(),
            title,
            artist,
            album,
            duration: dur,
            cover_path: cached_cover_path(path),
            cover: None,
        }
    } else {
        // lofty 失败：MP3 用 id3 回退，其它格式直接退化兜底
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        if ext.as_deref() == Some("mp3") {
            if let Some(t) = read_meta_id3(path, &fallback_title) {
                return t;
            }
        }
        eprintln!(
            "[扫描] read_meta 退化兜底（元数据缺失，仍加入列表）: {}",
            path.display()
        );
        Track {
            path: path.into(),
            title: fallback_title,
            artist: String::new(),
            album: String::new(),
            duration: 0.0,
            cover_path: cached_cover_path(path),
            cover: None,
        }
    }
}

/// 封面缓存路径是否已存在（lofty/id3 提取过的都会落盘到同一缓存路径）
fn cached_cover_path(path: &Path) -> Option<String> {
    let cp = cover_cache_path(path);
    if cp.exists() {
        cp.to_str().map(|s| s.to_string())
    } else {
        None
    }
}

/// lofty 解析失败时，对 MP3 用 `id3` crate 回退读取元数据。
/// 返回 None 表示 id3 也读不出（极少见，如文件完全损坏），调用方会退化为文件名兜底。
fn read_meta_id3(path: &Path, fallback_title: &str) -> Option<Track> {
    let tag = id3::Tag::read_from_path(path).ok()?;
    // 标题优先标准 TIT2；没有再回退文件名（TIT3 多为空或描述性，不优先）
    let title = tag
        .title()
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback_title.to_string());
    let artist = tag.artist().map(|s| s.to_string()).unwrap_or_default();
    let album = tag.album().map(|s| s.to_string()).unwrap_or_default();
    // 时长单独用 mp3_duration 估算（只扫帧头，不解析标签，损坏也不影响）
    let duration = mp3_duration::from_path(path)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    Some(Track {
        path: path.into(),
        title,
        artist,
        album,
        duration,
        cover_path: cached_cover_path(path),
        cover: None,
    })
}

/// 写回音频文件标签（标题/歌手/专辑）。
///
/// 主路径用 lofty：`read_from_path` 打开 → 改标签字段（Accessor 的 set_title 等）
/// → `save_to_path` 落盘。lofty 解析失败时（如 MINI姐 那批读都读不出的 MP3），
/// 若扩展名是 mp3，回退用 `id3` crate 写 ID3v2 标签（`Tag::write_to_path`）。
/// 返回错误信息供 UI 提示（文件被占用/只读等）。
pub fn write_meta(path: &Path, title: &str, artist: &str, album: &str) -> Result<(), String> {
    // 主路径：lofty 读写（MP3 写 ID3v2、FLAC 写 Vorbis、M4A 写 MP4，按文件类型自动处理）
    match write_meta_lofty(path, title, artist, album) {
        Ok(()) => return Ok(()),
        Err(lofty_err) => {
            // lofty 失败（读取阶段就失败的文件 lofty 也无法写）：若是 MP3 用 id3 回退
            let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
            if ext.as_deref() == Some("mp3") {
                if let Ok(()) = write_meta_id3(path, title, artist, album) {
                    return Ok(());
                }
            }
            Err(lofty_err)
        }
    }
}

/// 用 lofty 写标签（能覆盖 MP3/FLAC/M4A/OGG 等绝大多数格式）
fn write_meta_lofty(path: &Path, title: &str, artist: &str, album: &str) -> Result<(), String> {
    use lofty::config::WriteOptions;
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::tag::Accessor;

    let mut tagged = lofty::read_from_path(path).map_err(|e| format!("读取失败: {e}"))?;

    // 取第一个标签；若文件完全没有标签，新建一个该格式默认类型的标签
    if tagged.first_tag().is_none() {
        let tt = tagged.file_type().primary_tag_type();
        tagged.insert_tag(lofty::tag::Tag::new(tt));
    }
    let Some(tag) = tagged.first_tag_mut() else {
        return Err("该文件不支持写入标签".into());
    };

    // 空字符串 = 删除该字段，否则设置新值
    if title.is_empty() {
        tag.remove_title();
    } else {
        tag.set_title(title.to_string());
    }
    if artist.is_empty() {
        tag.remove_artist();
    } else {
        tag.set_artist(artist.to_string());
    }
    if album.is_empty() {
        tag.remove_album();
    } else {
        tag.set_album(album.to_string());
    }

    tagged
        .save_to_path(path, WriteOptions::default())
        .map_err(|e| format!("写入失败: {e}"))
}

/// 用 id3 crate 写 MP3 标签（lofty 读不出的带异常帧 MP3 也能写）
fn write_meta_id3(path: &Path, title: &str, artist: &str, album: &str) -> Result<(), String> {
    use id3::TagLike;

    // 能读到现有标签则在其上修改；读不到（全新文件）则新建空标签
    let mut tag = match id3::Tag::read_from_path(path) {
        Ok(t) => t,
        Err(_) => id3::Tag::new(),
    };
    if title.is_empty() {
        tag.remove_title();
    } else {
        tag.set_title(title.to_string());
    }
    if artist.is_empty() {
        tag.remove_artist();
    } else {
        tag.set_artist(artist.to_string());
    }
    if album.is_empty() {
        tag.remove_album();
    } else {
        tag.set_album(album.to_string());
    }
    tag.write_to_path(path, id3::Version::Id3v24)
        .map_err(|e| format!("写入失败: {e}"))
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

/// 扫描时用于告警的"疑似音频但不支持"的扩展名。
/// 仅用于诊断打印，避免把 jpg/txt/log 等普通文件也打出来刷屏。
const AUDIO_LIKE_EXTENSIONS: &[&str] = &[
    "opus", "wma", "ape", "m4b", "mid", "midi", "aiff", "aif",
    "dsf", "tta", "tak", "wv", "alac", "caf", "amr",
];

/// 扫描文件夹，提取所有音频文件
///
/// 诊断日志（stderr）：
/// - 扩展名命中白名单但 lofty 解析失败 → 打印"退化兜底"，但**仍加入列表**（字段用文件名/空值填充）
/// - 扩展名疑似音频但不在支持列表 → 提示需加白名单
/// - 其它非音频文件（jpg/txt/log 等）不打印，避免刷屏
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
                    let ext_l = ext.to_lowercase();
                    if AUDIO_EXTENSIONS.contains(&ext_l.as_str()) {
                        // 扩展名命中白名单：尽力读元数据，lofty 解析失败也兜底加入列表
                        // （标题用文件名、歌手/专辑留空、时长记 0），不会再有"少了歌"。
                        tracks.push(read_meta(&p));
                    } else if AUDIO_LIKE_EXTENSIONS.contains(&ext_l.as_str()) {
                        // 扩展名像音频但不在支持列表，提示用户需要加白名单
                        eprintln!("[扫描] 扩展名未支持（疑似音频，已跳过）: {} (.{})", p.display(), ext_l);
                    }
                    // 其它非音频文件（jpg/txt/log 等）不打印，避免刷屏
                }
            }
        }
    }
    eprintln!("[扫描] 完成：命中 {} 首，目录 {}", tracks.len(), dir.display());
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
    /// 跳转到指定秒数（rodio 0.20 起 Sink 支持 try_seek）
    fn seek(&mut self, secs: f64) {
        let _ = self.sink.try_seek(std::time::Duration::from_secs_f64(secs.max(0.0)));
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
        /// 设置音量（0.0 ~ 1.0）
        pub fn set_volume(&mut self, v: f64) {
            self.volume = v;
            self.inner.set_volume(v);
        }
        /// 跳转到指定秒数
        pub fn seek(&mut self, secs: f64) {
            self.inner.seek(secs);
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
    // WinRT 的时间跨度以 100ns 为单位（SetPosition 用）
    use windows::Foundation::TimeSpan;

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

        /// 设置音量（0.0 ~ 1.0）。记住值，之后换歌时新后端也沿用。
        pub fn set_volume(&mut self, v: f64) {
            self.volume = v.clamp(0.0, 1.0);
            if let Some(mp) = &self.media {
                let _ = mp.SetVolume(self.volume);
            }
            if let Some(r) = &mut self.rodio {
                r.set_volume(self.volume);
            }
        }

        /// 跳转到指定秒数。rodio 后端和 MediaPlayer 后端各走各的 API。
        pub fn seek(&mut self, secs: f64) {
            let secs = secs.max(0.0);
            if self.using_rodio {
                if let Some(r) = &mut self.rodio {
                    r.seek(secs);
                }
            } else if let Some(mp) = &self.media {
                // MediaPlaybackSession::SetPosition 接收 100ns 单位的 TimeSpan
                if let Ok(session) = mp.PlaybackSession() {
                    let ts = TimeSpan { Duration: (secs * 10_000_000.0) as i64 };
                    let _ = session.SetPosition(ts);
                }
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
