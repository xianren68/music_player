// ─── 系统媒体会话（OS 级"正在播放"集成）───
// Windows: 用 ISystemMediaTransportControlsInterop::GetForWindow(hwnd) 拿到
//   "绑定到本窗口"的 SMTC。这是 VLC / foobar2000 等播放器驱动系统音量弹出
//   "正在播放"卡片（悬停系统托盘音量图标那个）的标准做法。
//   之前踩过的坑：
//   1) souvlaki 走 GetForWindow 但拿到的是 hwnd:null（窗口还没创建），系统不弹卡片；
//   2) MediaPlayer.SystemMediaTransportControls() 是"独立"SMTC（不绑定窗口），
//      任务栏缩略图控件能出现，但音量弹出卡片（它按"正在播放音频的窗口会话"
//      来关联）不会弹出。所以必须用 GetForWindow(hwnd) 把 SMTC 绑到真实窗口。
// macOS / Linux: 仍用 souvlaki（MPRIS / MediaRemote），保持跨平台。
//
// 对外暴露统一的 MediaSession 接口；具体实现按平台分在 imp 模块里。
use std::ffi::c_void;
use std::sync::mpsc::Sender;

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use windows::core::*;
    use windows::Foundation::TypedEventHandler;
    use windows::Media::*;
    use windows::Storage::Streams::RandomAccessStreamReference;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;

    pub struct MediaSession {
        smtc: SystemMediaTransportControls,
        display_updater: SystemMediaTransportControlsDisplayUpdater,
    }

    impl MediaSession {
        pub fn new(event_tx: Sender<souvlaki::MediaControlEvent>, hwnd: *mut c_void) -> Option<MediaSession> {
            // 通过互操作接口拿到"绑定到本窗口"的 SMTC（返回运行时类，自带全部方法）。
            // 只有绑定窗口 + 正在播放音频的 SMTC，Windows 才会把它显示到
            // 任务栏缩略图控件 AND 音量弹出"正在播放"卡片里。
            let interop: ISystemMediaTransportControlsInterop =
                windows::core::factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>().ok()?;
            let hwnd = HWND(hwnd);
            let smtc: SystemMediaTransportControls =
                unsafe { interop.GetForWindow(hwnd) }.ok()?;

            let _ = smtc.SetIsEnabled(true);
            let _ = smtc.SetIsPlayEnabled(true);
            let _ = smtc.SetIsPauseEnabled(true);
            let _ = smtc.SetIsStopEnabled(true);
            let _ = smtc.SetIsNextEnabled(true);
            let _ = smtc.SetIsPreviousEnabled(true);
            let _ = smtc.SetIsFastForwardEnabled(true);
            let _ = smtc.SetIsRewindEnabled(true);

            let display_updater: SystemMediaTransportControlsDisplayUpdater = smtc.DisplayUpdater().ok()?;
            let _ = display_updater.SetType(MediaPlaybackType::Music);

            // 系统媒体键 / 任务栏控件点击 → 转发到 channel
            let tx = event_tx.clone();
            let handler = TypedEventHandler::new(
                move |_, args: windows::core::Ref<'_, SystemMediaTransportControlsButtonPressedEventArgs>| {
                    let args = match args.as_ref() {
                        Some(a) => a,
                        None => return Ok(()),
                    };
                    let btn = args.Button()?;
                    let evt = match btn {
                        SystemMediaTransportControlsButton::Play => souvlaki::MediaControlEvent::Play,
                        SystemMediaTransportControlsButton::Pause => {
                            souvlaki::MediaControlEvent::Pause
                        }
                        SystemMediaTransportControlsButton::Stop => souvlaki::MediaControlEvent::Stop,
                        SystemMediaTransportControlsButton::Next => souvlaki::MediaControlEvent::Next,
                        SystemMediaTransportControlsButton::Previous => {
                            souvlaki::MediaControlEvent::Previous
                        }
                        SystemMediaTransportControlsButton::FastForward => {
                            souvlaki::MediaControlEvent::Seek(souvlaki::SeekDirection::Forward)
                        }
                        SystemMediaTransportControlsButton::Rewind => {
                            souvlaki::MediaControlEvent::Seek(souvlaki::SeekDirection::Backward)
                        }
                        _ => return Ok(()),
                    };
                    let _ = tx.send(evt);
                    Ok(())
                },
            );
            smtc.ButtonPressed(&handler).ok()?;

            Some(MediaSession {
                smtc,
                display_updater,
            })
        }

        pub fn set_metadata(&mut self, title: &str, artist: &str, album: &str, cover_path: Option<&str>) {
            if let Ok(props) = self.display_updater.MusicProperties() {
                let _ = props.SetTitle(&HSTRING::from(title));
                let _ = props.SetArtist(&HSTRING::from(artist));
                let _ = props.SetAlbumTitle(&HSTRING::from(album));
            }
            // 封面：把本地 PNG 路径转成 file:// 后让 SMTC 自己加载
            if let Some(url) = build_cover_url(cover_path) {
                let path = url.trim_start_matches("file://");
                if let Ok(loader) = windows::Storage::StorageFile::GetFileFromPathAsync(&HSTRING::from(path)) {
                    if let Ok(file) = loader.get() {
                        if let Ok(stream) = RandomAccessStreamReference::CreateFromFile(&file) {
                            let _ = self.display_updater.SetThumbnail(&stream);
                        }
                    }
                }
            }
            if let Err(e) = self.display_updater.Update() {
                eprintln!("[media] display_updater.Update 失败: {e}");
            } else {
                eprintln!("[media] set_metadata 成功（含封面，Update 已调用）");
            }
        }

        pub fn set_playback(&mut self, playing: bool) {
            let status = if playing {
                MediaPlaybackStatus::Playing
            } else {
                MediaPlaybackStatus::Paused
            };
            if let Err(e) = self.smtc.SetPlaybackStatus(status) {
                eprintln!("[media] set_playback 失败: {e}");
            } else {
                eprintln!("[media] set_playback 成功: playing={}", playing);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;
    use souvlaki::{MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};

    pub struct MediaSession {
        controls: MediaControls,
    }

    impl MediaSession {
        pub fn new(event_tx: Sender<souvlaki::MediaControlEvent>, _hwnd: *mut c_void) -> Option<MediaSession> {
            let config = PlatformConfig {
                dbus_name: "gpui_music_player",
                display_name: "本地音乐播放器",
            };
            let mut controls = MediaControls::new(config).ok()?;
            controls
                .attach(move |evt| {
                    let _ = event_tx.send(evt);
                })
                .ok()?;
            Some(MediaSession { controls })
        }

        pub fn set_metadata(&mut self, title: &str, artist: &str, album: &str, cover_path: Option<&str>) {
            let cover_url = build_cover_url(cover_path);
            let cover_ref = cover_url.as_deref();
            if let Err(e) = self.controls.set_metadata(MediaMetadata {
                title: Some(title),
                artist: Some(artist),
                album: Some(album),
                cover_url: cover_ref,
                ..Default::default()
            }) {
                eprintln!("[media] set_metadata 失败: {e}");
                if cover_ref.is_some() {
                    if let Err(e2) = self.controls.set_metadata(MediaMetadata {
                        title: Some(title),
                        artist: Some(artist),
                        album: Some(album),
                        cover_url: None,
                        ..Default::default()
                    }) {
                        eprintln!("[media] set_metadata 无封面重试仍失败: {e2}");
                    }
                }
            } else {
                eprintln!("[media] set_metadata 成功（含封面，Update 已调用）");
            }
        }

        pub fn set_playback(&mut self, playing: bool) {
            let playback = if playing {
                MediaPlayback::Playing { progress: None }
            } else {
                MediaPlayback::Paused { progress: None }
            };
            if let Err(e) = self.controls.set_playback(playback) {
                eprintln!("[media] set_playback 失败: {e}");
            } else {
                eprintln!("[media] set_playback 成功: playing={}", playing);
            }
        }
    }
}

/// 跨平台统一对外接口
pub struct MediaSession {
    inner: imp::MediaSession,
}

impl MediaSession {
    /// 平台无关的创建入口。Windows 用 GetForWindow(hwnd) 拿到绑定窗口的 SMTC；
    /// 其他平台用 souvlaki。hwnd 在非 Windows 平台被忽略。平台不支持时返回 None。
    pub fn new(event_tx: Sender<souvlaki::MediaControlEvent>, hwnd: *mut c_void) -> Option<MediaSession> {
        imp::MediaSession::new(event_tx, hwnd).map(|inner| MediaSession { inner })
    }

    /// 更新正在播放的歌曲信息（标题 / 歌手 / 专辑 / 封面）
    pub fn set_metadata(&mut self, title: &str, artist: &str, album: &str, cover_path: Option<&str>) {
        self.inner.set_metadata(title, artist, album, cover_path);
    }

    /// 更新播放状态（播放 / 暂停）
    pub fn set_playback(&mut self, playing: bool) {
        self.inner.set_playback(playing);
    }
}

/// 把本地封面文件路径转成 SMTC 需要的 `file://<合法Windows绝对路径>` 形式。
/// - 去掉 canonicalize 产生的 `\\?\` / `\\?\UNC\` 前缀
/// - 只用两个斜杠 `file://`，后面紧跟盘符路径（SMTC 会 trim 掉 `file://` 当本地路径）
fn build_cover_url(cover_path: Option<&str>) -> Option<String> {
    let p = cover_path?;
    let canon = std::path::Path::new(p).canonicalize().ok()?;
    let s = canon.to_string_lossy().to_string();
    let clean = s
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{}", rest))
        .or_else(|| s.strip_prefix(r"\\?\").map(|rest| rest.to_string()))
        .unwrap_or(s);
    Some(format!("file://{}", clean))
}
