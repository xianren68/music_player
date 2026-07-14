// ─── 本地音乐播放器（gpui-component 版）───
mod models;
mod audio;
mod ui;
mod theme;
mod settings;
mod lyrics;
mod media_session;

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{slider::*};
use theme::ThemeConfig;
use settings::AppSettings;
use std::sync::Arc;
use std::sync::{Mutex, mpsc::{channel, Sender}};
use std::time::{Duration, Instant};
use souvlaki::MediaControlEvent;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use models::Folder;
use audio::Player;

/// 主题模式
#[derive(Clone, PartialEq)]
enum ThemeMode { Dark, Light }

/// 应用主结构
struct MusicPlayer {
    folders: Vec<Folder>,
    current: Option<(usize, usize)>,
    playing: bool,
    sidebar_open: bool,
    settings_open: bool,
    opacity_enabled: bool,
    /// 播放开始时间
    play_start: Option<Instant>,
    /// 暂停时已播放秒数
    play_offset: f64,
    player: Player,
    slider: Entity<SliderState>,
    _subscription: Subscription,
    /// 背景图透明度滑块
    bg_slider: Entity<SliderState>,
    _bg_sub: Subscription,
    /// 当前主题配置
    theme: ThemeConfig,
    /// 主题模式
    theme_mode: ThemeMode,
    /// 背景图片路径
    bg_image_path: Option<SharedString>,
    /// 背景图是否磨砂
    bg_image_blur: bool,
    /// 背景图纹理
    bg_image: Option<Arc<RenderImage>>,
    /// 音乐文件夹路径列表（支持多个）
    music_folder_paths: Vec<SharedString>,
    /// 正在加载的文件夹路径列表（用于显示 loading 状态）
    loading_folders: Vec<SharedString>,
    /// 当前播放曲目的专辑封面（渲染用）
    current_cover: Option<Arc<RenderImage>>,
    /// 正在后台加载封面的歌曲路径（防止重复触发懒加载）
    loading_covers: std::collections::HashSet<String>,
    /// 等待防抖后加载的封面列表 (fi, ti, 文件路径, 查找key)
    pending_cover_loads: Vec<(usize, usize, std::path::PathBuf, String)>,
    /// 封面加载防抖的代数（每次新渲染递增，定时器只处理最后一轮）
    cover_debounce_gen: u64,
    /// 均衡器动画相位（用于播放时三条竖条的高度跳动）
    eq_phase: u32,
    /// 系统媒体会话（任务栏/锁屏"正在播放"集成，souvlaki 封装）
    media: Option<crate::media_session::MediaSession>,
    /// 媒体会话事件发送端（render 首次拿到窗口句柄后用于重建会话）
    media_tx: Option<Sender<MediaControlEvent>>,
    /// 媒体会话是否已用窗口句柄初始化（避免 render 中重复创建）
    media_initialized: bool,
    /// 当前歌词
    lyrics: Option<lyrics::Lyrics>,
    /// 当前歌词行索引
    lyric_line: Option<usize>,
}

actions!(music_player, [ToggleSidebar, ToggleSettings, AddFolder, PlayPause, Next, Prev]);

impl MusicPlayer {
    fn new(cx: &mut Context<Self>) -> Self {
        // 加载持久化设置
        let saved = AppSettings::load();

        let slider = cx.new(|_| {
            SliderState::new()
                .min(0.1)
                .max(1.0)
                .step(0.01)
                .default_value(saved.opacity)
        });

        let subscription = cx.subscribe(&slider, move |_, _, _event: &SliderEvent, cx| {
            cx.notify();
        });

        // 背景图透明度的滑动条
        let bg_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .default_value(saved.bg_opacity)
        });

        let bg_sub = cx.subscribe(&bg_slider, move |_, _, _event: &SliderEvent, cx| {
            cx.notify();
        });

        // 根据保存的主题模式加载主题
        let theme = match saved.theme_mode.as_str() {
            "light" => ThemeConfig::light(),
            _ => ThemeConfig::dark(),
        };
        let theme_mode = match saved.theme_mode.as_str() {
            "light" => ThemeMode::Light,
            _ => ThemeMode::Dark,
        };

        // 加载背景图
        let bg_image = saved.bg_image_path.as_ref().and_then(|path| {
            std::fs::read(path).ok().and_then(|bytes| {
                image::load_from_memory(&bytes).ok().map(|img| {
                    let mut rgba = img.to_rgba8();
                    // GPUI 的 RenderImage 期望 BGRA 格式，需要交换 R/B 通道
                    for pixel in rgba.pixels_mut() {
                        pixel.0.swap(0, 2);
                    }
                    let frame = image::Frame::new(rgba);
                    Arc::new(RenderImage::new(
                        smallvec::SmallVec::from_elem(frame, 1)
                    ))
                })
            })
        });

        // 构建结构体
        let mut this = Self {
            folders: Vec::new(),
            current: None,
            playing: false,
            sidebar_open: saved.sidebar_open,
            settings_open: saved.settings_open,
            opacity_enabled: saved.opacity_enabled,
            play_start: None,
            play_offset: 0.0,
            player: Player::new(),
            slider,
            _subscription: subscription,
            bg_slider,
            _bg_sub: bg_sub,
            theme,
            theme_mode,
            bg_image_path: saved.bg_image_path.map(|s| s.into()),
            bg_image_blur: saved.bg_blur,
            bg_image,
            music_folder_paths: saved.music_folders.iter().map(|s| s.clone().into()).collect(),
            loading_folders: Vec::new(),
            current_cover: None,
            loading_covers: std::collections::HashSet::new(),
            pending_cover_loads: Vec::new(),
            cover_debounce_gen: 0,
            eq_phase: 0,
            media: None,
            media_tx: None,
            media_initialized: false,
            lyrics: None,
            lyric_line: None,
        };

        // 创建媒体会话事件通道，并启动监听循环。
        // 注意：Windows 上 SMTC 需要有效的窗口句柄(hwnd)才能显示，
        // 而 hwnd 只有在窗口创建后才能拿到，因此媒体会话本身推迟到
        // render 首次调用时（已有 w: &mut Window）再用真实 hwnd 创建。
        let (media_tx, media_rx) = channel::<MediaControlEvent>();
        this.media_tx = Some(media_tx);
        {
            // 用后台线程监听 channel，把系统媒体键事件转发回应用
            let rx = Arc::new(Mutex::new(media_rx));
            let rx2 = rx.clone();
            cx.spawn(async move |this, cx| {
                loop {
                    // 在后台线程阻塞等待事件，避免占用 GPUI 主执行器
                    let evt = cx.background_spawn({
                        let rx = rx2.clone();
                        async move { rx.lock().unwrap().recv().ok() }
                    }).await;
                    match evt {
                        Some(evt) => {
                            this.update(cx, |this, cx| this.handle_media_control(evt, cx)).ok();
                        }
                        None => break, // channel 已断开，退出循环
                    }
                }
            }).detach();
        }

        // 自动加载已保存的音乐目录（需要在 cx 可用后调用）
        // 清理旧版封面缓存文件（不带 v2_ 前缀的 80x80 低分辨率版本）
        crate::audio::cleanup_old_covers();
        this.load_music_folder(cx);
        this
    }

    /// 根据保存的路径自动加载音乐目录（优先使用缓存）
    fn load_music_folder(&mut self, cx: &mut Context<Self>) {
        if self.music_folder_paths.is_empty() {
            return;
        }

        // 优先从缓存加载（避免大目录每次都重新扫描）
        let saved = AppSettings::load();
        if !saved.cached_folders.is_empty() {
            eprintln!("[缓存] 从 settings.json 加载了 {} 个文件夹, {} 首歌曲", 
                saved.cached_folders.len(),
                saved.cached_folders.iter().map(|f| f.tracks.len()).sum::<usize>());
            self.folders = saved.cached_folders;
            // 验证 cover_path 有效性（旧版缓存文件可能已被清理）
            for folder in &mut self.folders {
                for track in &mut folder.tracks {
                    if let Some(ref cp) = track.cover_path {
                        if !std::path::Path::new(cp).exists() {
                            track.cover_path = None;
                        }
                    }
                }
            }
            return;
        }

        // 缓存为空时才扫描（在后台线程进行，避免卡顿）
        for path_str in &self.music_folder_paths {
            let path_str_clone = path_str.clone();
            let path_string = path_str_clone.to_string();
            let dir = std::path::Path::new(path_str.as_ref());
            if dir.is_dir() {
                eprintln!("[扫描] 缓存为空，正在后台扫描目录: {}", path_str);
                
                // 添加 loading 状态
                self.loading_folders.push(path_str.clone());
                
                // 在后台线程扫描
                cx.spawn(async move |this, cx| {
                    let path_string_clone = path_string.clone();
                    let tracks = cx.background_spawn(async move {
                        let dir = std::path::Path::new(&path_string_clone);
                        crate::audio::scan_dir(dir)
                    }).await;

                    // 扫描完成后更新 UI
                    this.update(cx, |this, cx| {
                        let dir = std::path::Path::new(&path_string);
                        let name = dir.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("音乐")
                            .to_string();
                        
                        if !tracks.is_empty() {
                            this.folders.push(Folder { name, expanded: true, tracks });
                        }
                        
                        // 移除 loading 状态
                        this.loading_folders.retain(|p| p != &path_str_clone);
                        
                        // 保存缓存
                        this.save_settings_for_cache();
                        cx.notify();
                    }).ok();
                }).detach();
            }
        }
    }


    /// 保存歌曲列表缓存
    fn save_settings_for_cache(&self) {
        let mut saved = AppSettings::load();
        saved.cached_folders = self.folders.clone();
        saved.music_folders = self.music_folder_paths.iter().map(|s| s.to_string()).collect();
        saved.save();
        eprintln!("[缓存] 已保存 {} 个文件夹到缓存", saved.cached_folders.len());
    }

    /// 保存当前设置到文件（包含歌曲列表缓存）
    fn save_settings(&self, cx: &Context<Self>) {
        let settings = AppSettings {
            opacity_enabled: self.opacity_enabled,
            opacity: self.slider.read(cx).value().start(),
            bg_opacity: self.bg_slider.read(cx).value().start(),
            bg_blur: self.bg_image_blur,
            bg_image_path: self.bg_image_path.as_ref().map(|s| s.to_string()),
            theme_mode: match self.theme_mode {
                ThemeMode::Light => "light".into(),
                _ => "dark".into(),
            },
            sidebar_open: self.sidebar_open,
            settings_open: self.settings_open,
            music_folders: self.music_folder_paths.iter().map(|s| s.to_string()).collect(),
            cached_folders: self.folders.clone(),  // 同步缓存
        };
        settings.save();
    }

    // ── 事件处理 ──

    fn toggle_sidebar(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        self.save_settings(cx);
        cx.notify();
    }

    fn toggle_settings(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        self.save_settings(cx);
        cx.notify();
    }

    fn play_pause(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.toggle_playback(cx);
    }

    /// 播放/暂停核心逻辑（供 UI 按钮和系统媒体键共用）
    fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if self.current.is_none() {
            if let Some((fi, ti)) = self.first() {
                self.play(fi, ti, cx);
            }
            return;
        }
        if self.playing {
            // 暂停：记录已播放时间
            self.play_offset = self.current_progress();
            self.play_start = None;
            self.player.toggle(true);
        } else {
            // 继续：重置开始时间
            self.play_start = Some(Instant::now());
            self.player.toggle(false);
        }
        self.playing = !self.playing;
        self.sync_media_session(cx);
        cx.notify();
    }

    fn next(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.next_track(cx);
    }

    /// 下一首核心逻辑（供 UI 按钮和系统媒体键共用）
    fn next_track(&mut self, cx: &mut Context<Self>) {
        if let Some((fi, ti)) = self.current {
            if let Some((f, t)) = self.next_idx(fi, ti) {
                self.play(f, t, cx);
                cx.notify();
            }
        }
    }

    fn prev(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.prev_track(cx);
    }

    /// 上一首核心逻辑（供 UI 按钮和系统媒体键共用）
    fn prev_track(&mut self, cx: &mut Context<Self>) {
        if let Some((fi, ti)) = self.current {
            if let Some((f, t)) = self.prev_idx(fi, ti) {
                self.play(f, t, cx);
                cx.notify();
            }
        }
    }

    /// 处理系统媒体指令（来自任务栏控件 / 锁屏 / 键盘媒体键）
    fn handle_media_control(&mut self, evt: MediaControlEvent, cx: &mut Context<Self>) {
        use MediaControlEvent::*;
        match evt {
            Play => { if !self.playing { self.toggle_playback(cx); } }
            Pause => { if self.playing { self.toggle_playback(cx); } }
            Toggle => { self.toggle_playback(cx); }
            Next => { self.next_track(cx); }
            Previous => { self.prev_track(cx); }
            Stop => {
                self.player.stop();
                self.playing = false;
                self.sync_media_session(cx);
            }
            _ => {}
        }
        cx.notify();
    }

    /// 把当前播放状态（歌名/歌手/专辑/封面/播放中）同步到系统媒体会话
    fn sync_media_session(&mut self, _cx: &mut Context<Self>) {
        let Some(media) = &mut self.media else {
            eprintln!("[media] sync: media 为 None，跳过（媒体会话未初始化）");
            return;
        };
        if let Some((fi, ti)) = self.current {
            if let Some(track) = self.folders.get(fi).and_then(|f| f.tracks.get(ti)) {
                media.set_metadata(&track.title, &track.artist, &track.album, track.cover_path.as_deref());
            } else {
                eprintln!("[media] sync: current=({fi},{ti}) 但 track 不存在");
            }
        } else {
            eprintln!("[media] sync: current 为 None（还没选歌），只更新播放状态");
        }
        media.set_playback(self.playing);
    }

    fn play_at(&mut self, fi: usize, ti: usize, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.play(fi, ti, cx);
        cx.notify();
    }

    fn toggle_folder(&mut self, fi: usize, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.folders.get_mut(fi) {
            f.expanded = !f.expanded;
            cx.notify();
        }
    }

    /// 最小化窗口
    fn minimize_window(&mut self, _: &ClickEvent, w: &mut Window, _cx: &mut Context<Self>) {
        w.minimize_window();
    }

    /// 切换最大化
    fn toggle_maximize(&mut self, _: &ClickEvent, w: &mut Window, _cx: &mut Context<Self>) {
        w.toggle_fullscreen();
    }

    /// 关闭窗口
    fn close_window(&mut self, _: &ClickEvent, w: &mut Window, _cx: &mut Context<Self>) {
        w.remove_window();
    }

    /// 选择背景图片
    fn pick_bg_image(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("选择背景图片")
            .add_filter("图片", &["png", "jpg", "jpeg", "bmp", "webp"])
            .pick_file()
        {
            let path_str: SharedString = path.to_string_lossy().into_owned().into();
            self.bg_image_path = Some(path_str.clone());
            self.theme.bg_image = Some(path_str);
            // 加载图片为纹理
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let mut rgba = img.to_rgba8();
                    // GPUI 的 RenderImage 期望 BGRA 格式，需要交换 R/B 通道
                    for pixel in rgba.pixels_mut() {
                        pixel.0.swap(0, 2);
                    }
                    let frame = image::Frame::new(rgba);
                    self.bg_image = Some(Arc::new(RenderImage::new(
                        smallvec::SmallVec::from_elem(frame, 1)
                    )));
                }
            }
            self.save_settings(cx);
            cx.notify();
        }
    }

    /// 清除背景图片
    fn clear_bg_image(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.bg_image_path = None;
        self.theme.bg_image = None;
        self.bg_image = None;
        self.save_settings(cx);
        cx.notify();
    }

    /// 切换背景图磨砂效果（Switch 回调）
    fn toggle_bg_blur(&mut self, checked: &bool, _w: &mut Window, cx: &mut Context<Self>) {
        self.bg_image_blur = *checked;
        self.save_settings(cx);
        cx.notify();
    }

    /// 选择音乐文件夹（在后台线程运行，避免阻塞主线程导致 RefCell panic）
    fn pick_music_folder(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        eprintln!("[文件夹] 点击了添加音乐文件夹按钮");
        let weak_entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            eprintln!("[文件夹] 正在打开文件选择对话框...");
            
            // 在独立线程中运行阻塞的文件对话框
            // rfd 在 Windows 上需要 COM 初始化，独立线程更可靠
            let (tx, rx) = std::sync::mpsc::channel::<Option<std::path::PathBuf>>();
            std::thread::spawn(move || {
                eprintln!("[文件夹] 后台线程：正在显示对话框");
                let result = rfd::FileDialog::new()
                    .set_title("选择音乐文件夹")
                    .pick_folder();
                eprintln!("[文件夹] 后台线程：选择结果 = {:?}", result);
                let _ = tx.send(result);
            });

            // 在后台等待结果
            let result = cx.background_spawn(async move {
                rx.recv().ok().flatten()
            }).await;

            eprintln!("[文件夹] 收到结果: {:?}", result);
            
            if let Some(dir) = result {
                let path_str: SharedString = dir.to_string_lossy().into_owned().into();
                
                // 检查是否已添加
                let already_exists = if let Some(entity) = weak_entity.upgrade() {
                    entity.read_with(cx, |this, _| {
                        this.music_folder_paths.contains(&path_str)
                    }).unwrap_or(false)
                } else {
                    false
                };

                if !already_exists {
                    // 添加 loading 状态
                    if let Some(entity) = weak_entity.upgrade() {
                        entity.update(cx, |this, cx| {
                            this.music_folder_paths.push(path_str.clone());
                            this.loading_folders.push(path_str.clone());
                            cx.notify();
                        }).ok();
                    }

                    eprintln!("[文件夹] 开始扫描文件夹: {}", path_str);
                    
                    // 在后台线程扫描文件夹
                    let path_string = path_str.to_string();
                    let path_str_clone = path_str.clone();
                    let weak_entity_clone = weak_entity.clone();
                    
                    cx.spawn(async move |cx| {
                        // 在后台线程执行扫描
                        let path_string_clone = path_string.clone();
                        let tracks = cx.background_spawn(async move {
                            let dir = std::path::Path::new(&path_string_clone);
                            crate::audio::scan_dir(dir)
                        }).await;

                        // 扫描完成后更新 UI
                        if let Some(entity) = weak_entity_clone.upgrade() {
                            entity.update(cx, |this, cx| {
                                let dir = std::path::Path::new(&path_string);
                                let name = dir.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("音乐")
                                    .to_string();
                                
                                if !tracks.is_empty() {
                                    this.folders.push(crate::models::Folder { 
                                        name: name.clone(), 
                                        expanded: true, 
                                        tracks 
                                    });
                                }
                                
                                // 移除 loading 状态
                                this.loading_folders.retain(|p| p != &path_str_clone);
                                
                                // 保存缓存
                                this.save_settings_for_cache();
                                this.save_settings(cx);
                                cx.notify();
                            }).ok();
                        }
                    }).detach();
                    
                    eprintln!("[文件夹] 文件夹扫描已启动");
                } else {
                    eprintln!("[文件夹] 文件夹已存在，跳过");
                }
            }
        }).detach();
    }

    /// 删除音乐文件夹
    fn remove_music_folder(&mut self, path: SharedString, cx: &mut Context<Self>) {
        self.music_folder_paths.retain(|p| p != &path);
        // 同时删除对应的文件夹
        let path_obj = std::path::Path::new(path.as_ref());
        let folder_name = path_obj.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        self.folders.retain(|f| f.name != folder_name);
        
        self.save_settings_for_cache();
        self.save_settings(cx);
        cx.notify();
    }

    /// 刷新音乐文件夹（在后台线程扫描）
    fn refresh_music_folder(&mut self, path: SharedString, cx: &mut Context<Self>) {
        // 添加 loading 状态
        self.loading_folders.push(path.clone());
        cx.notify();
        
        let path_clone = path.clone();
        let path_string = path_clone.to_string();
        cx.spawn(async move |this, cx| {
            // 在后台线程扫描
            let path_string_clone = path_string.clone();
            let tracks = cx.background_spawn(async move {
                let dir = std::path::Path::new(&path_string_clone);
                crate::audio::scan_dir(dir)
            }).await;

            // 扫描完成后更新 UI
            this.update(cx, |this, cx| {
                let dir = std::path::Path::new(&path_string);
                let name = dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("音乐")
                    .to_string();
                
                // 更新文件夹
                if let Some(folder) = this.folders.iter_mut().find(|f| f.name == name) {
                    folder.tracks = tracks;
                }
                
                // 移除 loading 状态
                this.loading_folders.retain(|p| p != &path_clone);
                
                // 保存缓存
                this.save_settings_for_cache();
                cx.notify();
            }).ok();
        }).detach();
    }

    // ── 播放控制 ──

    fn first(&self) -> Option<(usize, usize)> {
        self.folders.iter().enumerate()
            .find(|(_, f)| !f.tracks.is_empty())
            .map(|(i, _)| (i, 0))
    }

    fn next_idx(&self, fi: usize, ti: usize) -> Option<(usize, usize)> {
        let mut fi = fi;
        let mut ti = ti + 1;
        while fi < self.folders.len() {
            if ti < self.folders[fi].tracks.len() {
                return Some((fi, ti));
            }
            fi += 1;
            ti = 0;
        }
        None
    }

    fn prev_idx(&self, fi: usize, ti: usize) -> Option<(usize, usize)> {
        if ti > 0 { return Some((fi, ti - 1)); }
        if fi > 0 {
            let pf = fi - 1;
            if !self.folders[pf].tracks.is_empty() {
                return Some((pf, self.folders[pf].tracks.len() - 1));
            }
        }
        None
    }

    fn play(&mut self, fi: usize, ti: usize, cx: &mut Context<Self>) {
        self.player.stop();
        
        // 获取 track 的信息（先克隆需要的值，避免借用问题）
        let (track_path, track_title) = if let Some(track) = self.folders.get(fi).and_then(|f| f.tracks.get(ti)) {
            (track.path.clone(), track.title.clone())
        } else {
            return;
        };
        
        // 播放 track
        let track_ref = &self.folders[fi].tracks[ti];
        self.player.play(track_ref);
        
        // 在后台线程提取/加载专辑封面
        let track_path_clone = track_path.clone();
        let fi_clone = fi;
        let ti_clone = ti;
        cx.spawn(async move |this, cx| {
            // 在后台线程提取封面并缓存
            let cover_path = cx.background_spawn(async move {
                crate::audio::extract_and_cache_cover(&track_path_clone)
            }).await;
            
            // 更新 UI
            this.update(cx, |this, cx| {
                // 保存 cover_path 到 track
                if let Some(folder) = this.folders.get_mut(fi_clone) {
                    if let Some(track) = folder.tracks.get_mut(ti_clone) {
                        track.cover_path = cover_path.clone();
                    }
                }
                
                // 加载封面并转换为 RenderImage
                if let Some(ref cp) = cover_path {
                    if let Ok(cover_data) = std::fs::read(cp) {
                        if let Ok(img) = image::load_from_memory(&cover_data) {
                            let mut rgba = img.to_rgba8();
                            // GPUI 的 RenderImage 期望 BGRA 格式，需要交换 R/B 通道
                            for pixel in rgba.pixels_mut() {
                                pixel.0.swap(0, 2);
                            }
                            let frame = image::Frame::new(rgba);
                            this.current_cover = Some(Arc::new(RenderImage::new(
                                smallvec::SmallVec::from_elem(frame, 1)
                            )));
                        }
                    }
                } else {
                    this.current_cover = None;
                }
                
                // 保存缓存（持久化 cover_path）
                this.save_settings_for_cache();
                // 封面提取完成后，把封面也同步到系统媒体会话
                this.sync_media_session(cx);
                cx.notify();
            }).ok();
        }).detach();
        
        // 检查是否是同一首歌（避免重复加载歌词导致闪烁）
        let is_same_track = self.current == Some((fi, ti));
        
        self.current = Some((fi, ti));
        self.playing = true;
        self.play_offset = 0.0;
        self.play_start = Some(Instant::now());

        // 同步系统媒体会话（任务栏/锁屏"正在播放"）
        self.sync_media_session(cx);

        // 只有切换到不同歌曲时才加载歌词和重置位置
        if !is_same_track {
            self.load_lyrics(&track_path, &track_title);
            self.lyric_line = None;
        }
        
        self.spawn_progress_updater(cx);
        // 封面采用懒加载策略：侧边栏滚动到该歌曲时触发，不再批量预加载
    }
    
    /// 加载歌词
    fn load_lyrics(&mut self, track_path: &std::path::Path, title: &str) {
        // 查找歌词文件
        if let Some(lyrics) = lyrics::find_lyrics(track_path, title) {
            self.lyrics = Some(lyrics);
        } else {
            self.lyrics = None;
        }
    }

    /// 计算当前播放进度（秒）
    fn current_progress(&self) -> f64 {
        let elapsed = self.play_start
            .map(|s| s.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        (self.play_offset + elapsed).min(self.total_time())
    }

    /// 获取当前曲目总时长
    fn total_time(&self) -> f64 {
        self.current
            .and_then(|(fi, ti)| self.folders.get(fi).and_then(|f| f.tracks.get(ti)))
            .map(|t| t.duration)
            .unwrap_or(0.0)
    }

    fn spawn_progress_updater(&mut self, cx: &mut Context<Self>) {
        // 进度更新定时器（500ms）
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_spawn(async {
                    std::thread::sleep(Duration::from_millis(500));
                }).await;

                this.update(cx, |this, cx| {
                    if this.playing {
                        let progress = this.current_progress();
                        let total = this.total_time();
                        
                        // 更新歌词行索引
                        // 注意：find_line 返回 None 时不要重置 lyric_line，
                        // 否则会导致 UI 闪烁（在两句歌词之间或还没开始唱时）
                        if let Some(ref lyrics) = this.lyrics {
                            let progress_ms = (progress * 1000.0) as u32;
                            if let Some(new_line) = lyrics.find_line(progress_ms) {
                                if this.lyric_line != Some(new_line) {
                                    this.lyric_line = Some(new_line);
                                }
                            }
                            // find_line 返回 None 时保持 lyric_line 不变
                            // （在两句之间时不切换，还没开始唱时保持 None）
                        }
                        
                        if total > 0.0 && progress >= total {
                            this.playing = false;
                            this.play_offset = 0.0;
                            this.play_start = None;
                        }
                    }
                    cx.notify();
                }).ok();
            }
        }).detach();

        // 均衡器动画定时器（150ms 刷新一次，让竖条跳动）
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_spawn(async {
                    std::thread::sleep(Duration::from_millis(150));
                }).await;

                this.update(cx, |this, cx| {
                    if this.playing {
                        this.eq_phase = this.eq_phase.wrapping_add(1);
                        cx.notify();
                    }
                }).ok();
            }
        }).detach();
    }
}

// ── UI 渲染 ──

impl Render for MusicPlayer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_open = self.sidebar_open;
        let settings_open = self.settings_open;
        let opacity = self.slider.read(cx).value().start();
        let opacity_enabled = self.opacity_enabled;
        let t = self.theme.clone();
        let theme_mode = self.theme_mode.clone();

        // ── 系统媒体会话懒初始化 ──
        // Windows 用 ISystemMediaTransportControlsInterop::GetForWindow(hwnd) 拿到
        // 绑定本窗口的 SMTC（驱动任务栏缩略图控件 + 音量弹出"正在播放"卡片）。
        // 其他平台用 souvlaki。都在首次 render 时创建（此时已有真实窗口句柄，
        // 且 foreground 线程就绪、WinRT 可正常调用）。
        if !self.media_initialized {
            if let Some(tx) = &self.media_tx {
                // 取真实窗口句柄，用于把 SMTC 绑定到本窗口
                let hwnd = get_window_hwnd(w);
                if let Some(media) = crate::media_session::MediaSession::new(tx.clone(), hwnd) {
                    self.media = Some(media);
                    self.media_initialized = true;
                    eprintln!("[media] SMTC 会话已初始化（绑定窗口 hwnd={:?}）", hwnd);
                    self.sync_media_session(cx);
                } else {
                    eprintln!("[media] 警告：MediaSession::new 返回 None（SMTC 创建失败）");
                }
            }
        }

        // 获取当前曲目信息
        let (title, artist, album) = self.current
            .and_then(|(fi, ti)| self.folders.get(fi).and_then(|f| f.tracks.get(ti)))
            .map(|t| (t.title.clone(), t.artist.clone(), t.album.clone()))
            .unwrap_or_else(|| ("未播放".into(), "选择一首歌开始播放".into(), String::new()));

        let playing = self.playing;
        let cur_t = self.current_progress();
        let tot_t = self.total_time();
        let prog = if tot_t > 0.0 { cur_t / tot_t } else { 0.0 };
        let position_ms = (cur_t * 1000.0) as u32;

        // 计算侧边栏列表可用高度（窗口高度 - topbar - 标题栏 - tabs）
        let window_height = w.viewport_size().height;
        let sidebar_list_height = (window_height - px(200.0)).max(px(200.0));
        
        // 构建各模块
        let sidebar = ui::sidebar::build_sidebar(&self.folders, &self.loading_folders, &t, sidebar_list_height, cx);
        let topbar = ui::topbar::build_topbar(sidebar_open, settings_open, &t, cx);
        
        // 歌词数据（传递给 center）
        let lyrics_data = self.lyrics.as_ref();
        let lyric_line_idx = self.lyric_line;
        
        let center = ui::center::build_center(&title, &artist, &album, &t, cx, lyrics_data, lyric_line_idx, position_ms, self.current_cover.as_ref());
        let player_bar = ui::player::build_player_bar(playing, prog, cur_t, tot_t, &t, cx);
        let settings_panel = ui::settings::build_settings(
            opacity, opacity_enabled, &self.slider,
            &t, theme_mode,
            &self.bg_slider, self.bg_image_blur,
            cx,
        );

        // 背景图片层：放最底层
        let has_bg = self.bg_image.is_some();
        let bg_content_opacity = self.bg_slider.read(cx).value().start();
        let bg_layer = if let Some(ref img_tex) = self.bg_image {
            let img_alpha = if opacity_enabled { opacity } else { 1.0 };
            let blur_overlay = if self.bg_image_blur {
                div().absolute().top(px(0.0)).left(px(0.0)).size_full()
                    .bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 0.4 })
                    .into_any_element()
            } else {
                div().into_any_element()
            };
            div().absolute().top(px(0.0)).left(px(0.0)).size_full()
                .opacity(img_alpha)
                .child(img(img_tex.clone()).size_full().object_fit(ObjectFit::Cover))
                .child(blur_overlay)
                .into_any_element()
        } else {
            div().into_any_element()
        };

        // 内容区（不含 topbar）：侧边栏 + 主区域 + 设置面板
        let body = div().id("body").flex_1().min_h_0().flex().flex_row().overflow_hidden();
        let body = body
            .when(sidebar_open, |this| this.child(sidebar))
            // 主区域：center(封面+信息) + player_bar(进度条+控制) 整体垂直居中
            .child(div().id("main").flex_1().min_w_0()
                .flex().flex_col().items_center().justify_center()
                .child(center)
                .child(player_bar))
            .when(settings_open, |this| this.child(settings_panel));

        // 内容层：垂直排列 topbar + body
        // HTML 中 .app 用 --bg-surface (#12121a)，不是 --bg-base (#0a0a0f)
        let content = div().id("content").size_full().flex().flex_col();
        let content = if has_bg {
            content.bg(Hsla { h: t.surface.h, s: t.surface.s, l: t.surface.l, a: 1.0 - bg_content_opacity })
        } else if opacity_enabled {
            content.bg(t.surface).opacity(opacity)
        } else {
            content.bg(t.surface)
        };
        let content = content
            .child(topbar)
            .child(body);

        // 根容器
        let root = div().id("root").size_full().relative();
        root.child(bg_layer).child(content)
    }
}

// ── 系统媒体会话辅助 ──

/// 从 GPUI 的 Window 取出真实的 Win32 窗口句柄（HWND），用于把 SMTC 绑定到本窗口。
/// 返回原始指针；若取不到则返回 null。
fn get_window_hwnd(w: &Window) -> *mut std::ffi::c_void {
    // Window 自带一个返回 AnyWindowHandle 的同名固有方法，会遮蔽 trait 方法，
    // 这里显式用 HasWindowHandle::window_handle 拿到原始窗口句柄。
    match HasWindowHandle::window_handle(w) {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32) => win32.hwnd.get() as *mut std::ffi::c_void,
            _ => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

// ── 资源加载 ──

struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>, anyhow::Error> {
        // 尝试多个路径查找 assets
        let candidates = [
            // 当前工作目录（cargo run 时是项目根目录）
            Some(std::path::PathBuf::from("assets").join(path)),
            // 可执行文件所在目录
            std::env::current_exe().ok()
                .and_then(|p| p.parent().map(|d| d.join("assets").join(path))),
            // 可执行文件的上两级（target/debug/xxx.exe → 项目根目录）
            std::env::current_exe().ok()
                .and_then(|p| p.parent()
                    .and_then(|d| d.parent())
                    .and_then(|d| d.parent())
                    .map(|d| d.join("assets").join(path))),
        ];
        for candidate in candidates.into_iter().flatten() {
            if let Ok(data) = std::fs::read(&candidate) {
                return Ok(Some(std::borrow::Cow::Owned(data)));
            }
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>, anyhow::Error> {
        let candidates = [
            Some(std::path::PathBuf::from("assets").join(path)),
            std::env::current_exe().ok()
                .and_then(|p| p.parent().map(|d| d.join("assets").join(path))),
            std::env::current_exe().ok()
                .and_then(|p| p.parent()
                    .and_then(|d| d.parent())
                    .and_then(|d| d.parent())
                    .map(|d| d.join("assets").join(path))),
        ];
        for candidate in candidates.into_iter().flatten() {
            if let Ok(entries) = std::fs::read_dir(&candidate) {
                return Ok(entries
                    .filter_map(|entry| {
                        entry.ok().and_then(|e| {
                            e.file_name().into_string().ok().map(SharedString::from)
                        })
                    })
                    .collect());
            }
        }
        Ok(Vec::new())
    }
}

// ── 入口 ──

fn main() {
    Application::new()
        .with_assets(Assets)
        .run(move |cx| {
        gpui_component::init(cx);
        cx.spawn(async move |cx| {
            let _ = cx.open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("本地音乐播放器".into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    window_background: WindowBackgroundAppearance::Transparent,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(100.0), px(100.0)),
                        size(px(1000.0), px(650.0)),
                    ))),
                    ..Default::default()
                },
                |_window, cx| {
                    cx.new(|cx| MusicPlayer::new(cx))
                },
            );
        })
        .detach();
    });
}
