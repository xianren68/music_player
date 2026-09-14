// ─── 本地音乐播放器（gpui-component 版）───
mod models;
mod audio;
mod ui;
mod theme;
mod settings;
mod lyrics;
mod media_session;
mod platform;
mod tray;

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{slider::*, Root};
use gpui_component::input::{Input, InputState, InputEvent};
use theme::ThemeConfig;
use settings::AppSettings;
use std::sync::Arc;
use std::sync::{Mutex, mpsc::{channel, Sender}};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use souvlaki::MediaControlEvent;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tray::TrayEvent;

use models::Folder;
use audio::Player;
use crate::ui::sidebar::ListView;

/// 主题模式
#[derive(Clone, PartialEq)]
enum ThemeMode { Dark, Light }

/// 循环播放模式：顺序播放 / 列表循环 / 单曲循环
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode { Off, All, One }

impl RepeatMode {
    /// 持久化用的字符串
    fn as_str(self) -> &'static str {
        match self {
            RepeatMode::Off => "off",
            RepeatMode::All => "all",
            RepeatMode::One => "one",
        }
    }
    /// 从持久化字符串还原（非法值回退为顺序播放）
    fn from_str(s: &str) -> Self {
        match s {
            "all" => RepeatMode::All,
            "one" => RepeatMode::One,
            _ => RepeatMode::Off,
        }
    }
    /// 设置面板里显示的文案
    pub fn label(self) -> &'static str {
        match self {
            RepeatMode::Off => "顺序播放",
            RepeatMode::All => "列表循环",
            RepeatMode::One => "单曲循环",
        }
    }
}

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
    /// 音量滑块（设置面板「播放」分组）
    volume_slider: Entity<SliderState>,
    _volume_sub: Subscription,
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
    /// 搜索输入框状态（懒初始化，render 首次拿到窗口句柄后创建）
    search_input: Option<Entity<InputState>>,
    /// 搜索输入框的变更订阅（必须持有，否则渲染结束后订阅被释放、搜索失效）
    search_sub: Option<Subscription>,
    /// 当前搜索关键字（非空时侧边栏切换为扁平搜索结果视图）
    search_query: String,
    /// 正在编辑元数据的曲目位置 (fi, ti)；None = 未打开编辑弹窗
    edit_target: Option<(usize, usize)>,
    /// 编辑弹窗：标题输入框（懒初始化，打开弹窗时预填当前值）
    edit_title: Option<Entity<InputState>>,
    /// 编辑弹窗：歌手输入框
    edit_artist: Option<Entity<InputState>>,
    /// 编辑弹窗：专辑输入框
    edit_album: Option<Entity<InputState>>,
    /// 编辑保存失败的错误信息（显示在弹窗内）
    edit_error: Option<String>,
    /// 当前悬停的曲目位置 (fi, ti)：驱动该行 ⋯ 编辑按钮的展开/收起。
    /// 按钮始终在元素树里（只变宽度/透明度样式，不增删节点，避免 GPUI 绘制状态机 panic）
    hovered_track: Option<(usize, usize)>,
    /// 侧边栏当前展示的列表视图（全部 / 播放列表 / 专辑 / 歌手）
    active_tab: ListView,
    /// 专辑视图中已展开的分组 key 集合（key = 专辑名，空串代表"未知专辑"）
    expanded_albums: HashSet<String>,
    /// 歌手视图中已展开的分组 key 集合（key = 歌手名，空串代表"未知歌手"）
    expanded_artists: HashSet<String>,
    /// 随机播放是否开启
    shuffle: bool,
    /// 循环模式（顺序 / 列表循环 / 单曲循环）
    repeat: RepeatMode,
    /// 音量 (0.0 ~ 1.0)
    volume: f64,
    /// 进度更新定时器是否已启动：整个应用只跑一个循环即可，
    /// 否则每 play 一次就多一个 500ms 定时器（旧实现的问题）
    progress_timer_started: bool,
    /// 播放队列面板是否展开（第三组 UI）
    queue_open: bool,
    /// 最近播放历史（第三组 UI）：play() 时把曲目推到最前，去重、上限 50 条
    recent: Vec<(usize, usize)>,
    /// 专辑 tab 是否用封面墙（网格）而不是列表（第三组 UI）
    album_grid: bool,
    /// 歌词字号档位 -1~2（第三组 UI：只影响显示）
    lyric_font_step: i32,
    /// 歌词是否居中显示（第三组 UI：关掉则左对齐）
    lyric_centered: bool,
    /// 迷你播放器模式（第四组：紧凑单行布局 + 窗口缩小）
    mini_mode: bool,
    /// 进迷你模式前的窗口尺寸，退出时还原
    normal_size: Option<Size<Pixels>>,
    /// 文件关联注册结果提示（显示在设置面板）
    assoc_msg: Option<String>,
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

        // 音量滑块：拖动即应用到播放后端并持久化
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(1.0)
                .step(0.01)
                .default_value(saved.volume)
        });

        let volume_sub = cx.subscribe(&volume_slider, move |this, _, _event: &SliderEvent, cx| {
            this.on_volume_change(cx);
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
            volume_slider,
            _volume_sub: volume_sub,
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
            search_input: None,
            search_sub: None,
            search_query: String::new(),
            edit_target: None,
            edit_title: None,
            edit_artist: None,
            edit_album: None,
            edit_error: None,
            hovered_track: None,
            active_tab: ListView::Playlists,
            expanded_albums: HashSet::new(),
            expanded_artists: HashSet::new(),
            // 随机/循环/音量从设置里恢复
            shuffle: saved.shuffle,
            repeat: RepeatMode::from_str(&saved.repeat),
            volume: saved.volume as f64,
            progress_timer_started: false,
            // 第三组 UI 的初始状态
            queue_open: false,
            recent: Vec::new(),
            album_grid: false,
            lyric_font_step: 0,
            lyric_centered: true,
            // 第四组
            mini_mode: false,
            normal_size: None,
            assoc_msg: None,
        };

        // 把恢复的音量应用到播放后端
        this.player.set_volume(this.volume);

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

        // ── 系统托盘（第四组）──
        // start_tray 会在后台线程挂图标，这里只负责把托盘事件搬回主线程处理。
        if let Some(rx) = crate::tray::start_tray() {
            let rx = Arc::new(Mutex::new(rx));
            let rx2 = rx.clone();
            cx.spawn(async move |this, cx| {
                loop {
                    // 在后台线程阻塞等待，避免占用 GPUI 主执行器
                    let evt = cx.background_spawn({
                        let rx = rx2.clone();
                        async move { rx.lock().unwrap().recv().ok() }
                    }).await;
                    match evt {
                        Some(TrayEvent::Quit) => {
                            this.update(cx, |_this, cx| cx.quit()).ok();
                            break;
                        }
                        Some(evt) => {
                            this.update(cx, |this, cx| this.handle_tray_event(evt, cx)).ok();
                        }
                        None => break, // 托盘线程退出
                    }
                }
            }).detach();
        }

        // ── 第二个实例送来的"要打开的文件"轮询（第四组 单实例 + 文件关联）──
        // 用每秒一次的轮询（而不是 IPC）：第二个实例把文件路径写进 %TEMP% 的一个文本文件，
        // 这里读到就播、并立刻删掉请求文件。第一次轮询要等 1s，正好避开启动扫描。
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_spawn(async {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }).await;
                let pending = cx.background_spawn(async {
                    crate::tray::take_pending_file()
                }).await;
                if let Some(path) = pending {
                    this.update(cx, |this, cx| this.open_external_file(path, cx)).ok();
                }
                // 页面/视图被丢弃后 this.update 会失败，借此退出循环
                if this.update(cx, |_, _| {}).is_err() {
                    break;
                }
            }
        }).detach();

        this
    }

    /// 托盘菜单事件 → 播放器动作
    fn handle_tray_event(&mut self, evt: TrayEvent, cx: &mut Context<Self>) {
        match evt {
            TrayEvent::PlayPause => self.toggle_playback(cx),
            TrayEvent::Next => self.next_track(cx),
            TrayEvent::Prev => self.prev_track(cx),
            TrayEvent::ToggleWindow => {
                // 窗口操作要 Window 句柄，从 App 里取当前窗口再进去操作
                if let Some(handle) = cx.windows().into_iter().next() {
                    let _ = cx.update_window(handle, |_, w, _cx| {
                        let hwnd = get_window_hwnd(w) as isize;
                        crate::platform::toggle_window(hwnd);
                    });
                }
            }
            TrayEvent::Quit => cx.quit(),
        }
        cx.notify();
    }

    /// 打开一个外部音频文件（命令行参数 / 第二个实例转发 / 文件关联双击都走这里）。
    /// 库里已有同路径就直接播；没有就读元数据塞进"外部文件"文件夹再播。
    fn open_external_file(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        // 1) 库里找得到 → 直接播
        for (fi, folder) in self.folders.iter().enumerate() {
            for (ti, track) in folder.tracks.iter().enumerate() {
                if track.path == path {
                    self.play(fi, ti, cx);
                    return;
                }
            }
        }
        // 2) 不在库里 → 读标签建一个 Track，放进"外部文件"文件夹
        if !path.exists() {
            eprintln!("[open] 文件不存在：{}", path.display());
            return;
        }
        let track = crate::audio::read_meta(&path);
        let fi = if let Some(i) = self.folders.iter().position(|f| f.name == "外部文件") {
            i
        } else {
            self.folders.insert(0, Folder {
                name: "外部文件".to_string(),
                expanded: true,
                tracks: Vec::new(),
            });
            0
        };
        self.folders[fi].tracks.push(track);
        self.folders[fi].expanded = true;
        let ti = self.folders[fi].tracks.len() - 1;
        eprintln!("[open] 外部文件已加入库并开始播放：{}", path.display());
        self.play(fi, ti, cx);
    }

    /// 切换迷你播放器模式：窗口缩到 420×120（紧凑单行布局），退出时还原原尺寸
    fn toggle_mini_mode(&mut self, _: &ClickEvent, w: &mut Window, cx: &mut Context<Self>) {
        self.mini_mode = !self.mini_mode;
        if self.mini_mode {
            // 记住当前尺寸，退出时还原
            self.normal_size = Some(w.bounds().size);
            w.resize(size(px(420.0), px(120.0)));
        } else if let Some(sz) = self.normal_size.take() {
            w.resize(sz);
        }
        cx.notify();
    }

    /// 注册文件关联（写 HKCU 注册表），结果写进 assoc_msg 供设置面板显示
    fn register_association(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.assoc_msg = Some(match crate::tray::register_file_association() {
            Ok(()) => "已注册：右键音频文件 → 打开方式里能看到本程序".to_string(),
            Err(e) => format!("注册失败：{e}"),
        });
        cx.notify();
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
            shuffle: self.shuffle,
            repeat: self.repeat.as_str().to_string(),
            volume: self.volume as f32,
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

    /// 「播放列表」视图里展开/收起某个文件夹
    fn toggle_folder(&mut self, fi: usize, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.folders.get_mut(fi) {
            f.expanded = !f.expanded;
            cx.notify();
        }
    }

    /// 切换侧边栏列表视图（播放列表 / 专辑 / 歌手）
    fn set_tab(&mut self, tab: ListView, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if self.active_tab != tab {
            self.active_tab = tab;
            cx.notify();
        }
    }

    /// 切换随机播放
    fn toggle_shuffle(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.shuffle = !self.shuffle;
        self.save_settings(cx);
        cx.notify();
    }

    /// 展开/收起播放队列面板（第三组 UI）
    fn toggle_queue(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.queue_open = !self.queue_open;
        cx.notify();
    }

    /// 歌词字号档位增减（第三组 UI：只影响显示，不落盘）
    fn lyric_font_step_change(&mut self, delta: i32, cx: &mut Context<Self>) {
        self.lyric_font_step = (self.lyric_font_step + delta).clamp(-1, 2);
        cx.notify();
    }

    /// 切换歌词居中 / 左对齐（第三组 UI）
    fn toggle_lyric_align(&mut self, cx: &mut Context<Self>) {
        self.lyric_centered = !self.lyric_centered;
        cx.notify();
    }

    /// 记录一次播放到"最近播放"（置顶去重，最多留 50 条）
    fn push_recent(&mut self, fi: usize, ti: usize) {
        self.recent.retain(|&(f, t)| !(f == fi && t == ti));
        self.recent.insert(0, (fi, ti));
        self.recent.truncate(50);
    }

    /// 取"接下来"的曲目列表（队列面板用）：从当前曲目往后按顺序取，最多 30 首。
    /// 注意：这里刻意不看随机/循环模式，面板只是展示"顺序上的下一批"，避免和播放逻辑耦合。
    fn upcoming_list(&self, limit: usize) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        if let Some((mut fi, mut ti)) = self.current {
            while out.len() < limit {
                match self.next_sequential(fi, ti) {
                    Some((nf, nt)) => {
                        fi = nf; ti = nt;
                        if let Some(track) = self.folders.get(fi).and_then(|f| f.tracks.get(ti)) {
                            out.push((
                                track.title.clone(),
                                track.artist.clone(),
                                crate::audio::fmt_time(track.duration),
                            ));
                        }
                    }
                    None => break,
                }
            }
            // 开了列表循环时，把开头几首补到队尾，让面板看起来是"会循环的队列"
            if self.repeat == RepeatMode::All && !out.is_empty() {
                'outer: for (f, folder) in self.folders.iter().enumerate() {
                    for (t, track) in folder.tracks.iter().enumerate() {
                        if out.len() >= limit { break 'outer; }
                        // 跳过当前曲目自己
                        if Some((f, t)) == self.current { continue; }
                        out.push((
                            track.title.clone(),
                            track.artist.clone(),
                            crate::audio::fmt_time(track.duration),
                        ));
                    }
                }
            }
        }
        out
    }

    /// 循环切换循环模式：顺序播放 → 列表循环 → 单曲循环 → 顺序播放
    fn cycle_repeat(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.repeat = match self.repeat {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        self.save_settings(cx);
        cx.notify();
    }

    /// 音量滑块变化：读到新音量 → 应用到播放后端 → 持久化
    fn on_volume_change(&mut self, cx: &mut Context<Self>) {
        self.volume = self.volume_slider.read(cx).value().start() as f64;
        self.player.set_volume(self.volume);
        self.save_settings(cx);
        cx.notify();
    }

    /// 按比例跳转（进度条点击/拖拽：把点击位置换算成 0~1 的比例传进来）
    fn seek_fraction(&mut self, frac: f32, cx: &mut Context<Self>) {
        let total = self.total_time();
        if total <= 0.0 { return; }
        let target = (frac.clamp(0.0, 1.0) as f64) * total;
        self.seek_to(target, cx);
    }

    /// 跳转到指定秒数：调用后端 seek，并把本地的进度基准同步过去，
    /// 否则定时器算出来的进度会立刻跳回旧位置。
    fn seek_to(&mut self, secs: f64, cx: &mut Context<Self>) {
        let total = self.total_time();
        let mut target = secs.max(0.0);
        if total > 0.0 && target > total {
            target = total;
        }
        self.player.seek(target);
        // 本地进度基准 = 新位置，UI 立即反映
        self.play_offset = target;
        self.play_start = if self.playing { Some(Instant::now()) } else { None };
        // 歌词行也跟着跳
        if let Some(ref lyrics) = self.lyrics {
            if let Some(line) = lyrics.find_line((target * 1000.0) as u32) {
                self.lyric_line = Some(line);
            }
        }
        cx.notify();
    }

    /// 打开"编辑歌曲信息"弹窗，预填当前曲目的标题/歌手/专辑
    fn open_edit(&mut self, fi: usize, ti: usize, w: &mut Window, cx: &mut Context<Self>) {
        // 懒初始化三个输入框（InputState::new 需要窗口句柄）
        if self.edit_title.is_none() {
            let title = cx.new(|cx2| InputState::new(w, cx2));
            let artist = cx.new(|cx2| InputState::new(w, cx2));
            let album = cx.new(|cx2| InputState::new(w, cx2));
            self.edit_title = Some(title);
            self.edit_artist = Some(artist);
            self.edit_album = Some(album);
        }
        // 预填当前值
        if let Some(track) = self.folders.get(fi).and_then(|f| f.tracks.get(ti)) {
            let t = track.title.clone();
            let a = track.artist.clone();
            let al = track.album.clone();
            if let Some(input) = &self.edit_title {
                input.update(cx, |state, cx| state.set_value(t, w, cx));
            }
            if let Some(input) = &self.edit_artist {
                input.update(cx, |state, cx| state.set_value(a, w, cx));
            }
            if let Some(input) = &self.edit_album {
                input.update(cx, |state, cx| state.set_value(al, w, cx));
            }
        }
        self.edit_target = Some((fi, ti));
        self.edit_error = None;
        cx.notify();
    }

    /// 关闭编辑弹窗（不保存）
    fn close_edit(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.edit_target = None;
        self.edit_error = None;
        cx.notify();
    }

    /// 保存编辑：写回音频文件标签 → 更新内存 Track → 同步缓存 → 关闭弹窗
    fn save_edit(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let Some((fi, ti)) = self.edit_target else { return };
        let Some(track) = self.folders.get(fi).and_then(|f| f.tracks.get(ti)) else { return };
        let path = track.path.clone();
        let title = self.edit_title.as_ref().map(|i| i.read(cx).value().to_string()).unwrap_or_default();
        let artist = self.edit_artist.as_ref().map(|i| i.read(cx).value().to_string()).unwrap_or_default();
        let album = self.edit_album.as_ref().map(|i| i.read(cx).value().to_string()).unwrap_or_default();

        // 写回音频文件标签（lofty 主路径 + MP3 的 id3 回退）
        match crate::audio::write_meta(&path, &title, &artist, &album) {
            Ok(()) => {
                // 更新内存中的 Track（列表、搜索索引、播放信息随之刷新）
                if let Some(track) = self.folders.get_mut(fi).and_then(|f| f.tracks.get_mut(ti)) {
                    track.title = title;
                    track.artist = artist;
                    track.album = album;
                }
                // 同步系统媒体会话（如果正在播放这首歌，任务栏卡片也要更新）
                self.sync_media_session(cx);
                // 同步 settings.json 缓存，避免重启后显示旧值
                self.save_settings_for_cache();
                self.edit_target = None;
                self.edit_error = None;
                eprintln!("[编辑] 已保存元数据: {}", path.display());
            }
            Err(e) => {
                eprintln!("[编辑] 保存失败: {}", e);
                self.edit_error = Some(e);
            }
        }
        cx.notify();
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

    /// 所有可播放曲目的 (fi, ti) 扁平列表（随机播放用）
    fn all_indices(&self) -> Vec<(usize, usize)> {
        let mut v = Vec::new();
        for (fi, f) in self.folders.iter().enumerate() {
            for ti in 0..f.tracks.len() {
                v.push((fi, ti));
            }
        }
        v
    }

    /// 随机取一首（尽量避开当前这首）。用系统时间纳秒做种子，不引第三方随机库。
    fn random_idx(&self, cur: Option<(usize, usize)>) -> Option<(usize, usize)> {
        let all = self.all_indices();
        if all.is_empty() { return None; }
        if all.len() == 1 { return Some(all[0]); }
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize + d.as_secs() as usize)
            .unwrap_or(0);
        let mut idx = seed % all.len();
        if Some(all[idx]) == cur {
            // 恰好又随机到当前这首 → 顺延一首，保证"下一首"一定换歌
            idx = (idx + 1) % all.len();
        }
        Some(all[idx])
    }

    /// 最后一首（列表循环时"最后一首的下一首"回到开头用得到）
    fn last(&self) -> Option<(usize, usize)> {
        for fi in (0..self.folders.len()).rev() {
            let n = self.folders[fi].tracks.len();
            if n > 0 { return Some((fi, n - 1)); }
        }
        None
    }

    /// 顺序取下一首（不处理循环）
    fn next_sequential(&self, fi: usize, ti: usize) -> Option<(usize, usize)> {
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

    /// 顺序取上一首（不处理循环）
    fn prev_sequential(&self, fi: usize, ti: usize) -> Option<(usize, usize)> {
        if ti > 0 { return Some((fi, ti - 1)); }
        if fi > 0 {
            let pf = fi - 1;
            if !self.folders[pf].tracks.is_empty() {
                return Some((pf, self.folders[pf].tracks.len() - 1));
            }
        }
        None
    }

    /// 下一首：随机优先；否则顺序；顺序到头且开了列表循环就回到第一首
    fn next_idx(&self, fi: usize, ti: usize) -> Option<(usize, usize)> {
        if self.shuffle { return self.random_idx(Some((fi, ti))); }
        if let Some(next) = self.next_sequential(fi, ti) { return Some(next); }
        if self.repeat == RepeatMode::All { return self.first(); }
        None
    }

    /// 上一首：随机优先；否则顺序；顺序到头且开了列表循环就跳到最后一首
    fn prev_idx(&self, fi: usize, ti: usize) -> Option<(usize, usize)> {
        if self.shuffle { return self.random_idx(Some((fi, ti))); }
        if let Some(prev) = self.prev_sequential(fi, ti) { return Some(prev); }
        if self.repeat == RepeatMode::All { return self.last(); }
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
        // 记一笔播放历史（第三组 UI 的「最近」tab 用）
        self.push_recent(fi, ti);
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
        // 定时器循环只启动一次：整个应用跑一个 500ms 进度循环 + 一个 150ms 均衡器循环即可。
        // 旧实现每次 play 都新起两个循环，切歌多了会累积出几十个定时器在空转。
        if self.progress_timer_started {
            return;
        }
        self.progress_timer_started = true;

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
                        
                        // ── 播完一首的处理（这是"循环模式"真正生效的地方）──
                        if total > 0.0 && progress >= total {
                            match this.repeat {
                                // 单曲循环：重放当前这首
                                RepeatMode::One => {
                                    if let Some((fi, ti)) = this.current {
                                        this.play(fi, ti, cx);
                                    } else {
                                        this.playing = false;
                                    }
                                }
                                // 顺序 / 列表循环：都走"下一首"，
                                // next_idx 内部已处理"列表循环时回绕到第一首"
                                _ => {
                                    let cur = this.current;
                                    let next = match cur {
                                        Some((fi, ti)) => this.next_idx(fi, ti),
                                        None => None,
                                    };
                                    match next {
                                        Some((f, t)) => this.play(f, t, cx),
                                        // 已是最后一首且没开列表循环 → 停在末尾
                                        None => {
                                            this.playing = false;
                                            this.play_offset = 0.0;
                                            this.play_start = None;
                                        }
                                    }
                                }
                            }
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

    /// 构建"编辑歌曲信息"弹窗：全屏半透明遮罩 + 居中面板（标题/歌手/专辑输入框 + 按钮）
    fn build_edit_modal(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = self.theme.clone();
        // 输入框应已由 open_edit 懒初始化；若缺失（理论上不会）则安全降级为不渲染
        let Some(title) = self.edit_title.clone() else { return div().into_any_element() };
        let Some(artist) = self.edit_artist.clone() else { return div().into_any_element() };
        let Some(album) = self.edit_album.clone() else { return div().into_any_element() };
        let error = self.edit_error.clone();

        // 字段输入框（复用搜索框同一套 Input 组件）；label 转成 owned String，
        // 避免 &str 参数在闭包内逃逸（div().child() 要求 'static 元素）
        let field = |label: String, input: &Entity<InputState>| {
            div().flex_col().gap_1()
                .child(div().text_xs().text_color(t.muted_fg).child(label))
                .child(Input::new(input)
                    .bordered(true)
                    .appearance(false)
                    .px_2().py_1().rounded_md())
        };

        // 遮罩：点击空白处关闭弹窗（需先 .id() 才有 on_click，见 StatefulInteractiveElement）
        let overlay = div().id("edit-overlay").absolute().top(px(0.0)).left(px(0.0)).size_full()
            .bg(Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.55 })
            .flex().items_center().justify_center()
            .on_click(cx.listener(|this, e, w, cx| this.close_edit(e, w, cx)));

        // 错误提示（保存失败时显示，提前构建为 owned String 避免借用逃逸）
        let error_el = error.map(|e| {
            div().text_xs().text_color(Hsla { h: 0.0, s: 0.8, l: 0.55, a: 1.0 }).child(e)
        });

        let mut panel = div().id("edit-panel").w(px(360.0)).rounded_lg().p_4().flex_col().gap_3()
            .bg(t.surface).border_1().border_color(t.border)
            // 点击面板内部不冒泡到遮罩（避免误关）
            .on_click(cx.listener(|_this, _e, _w, cx| cx.stop_propagation()))
            // 标题行
            .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.fg).child("编辑歌曲信息"))
            // 三个字段
            .child(field("标题".to_string(), &title))
            .child(field("歌手".to_string(), &artist))
            .child(field("专辑".to_string(), &album));
        if let Some(el) = error_el {
            panel = panel.child(el);
        }
        let panel = panel
            // 按钮行：取消 / 保存
            .child(div().flex().justify_end().gap_2().pt_1()
                .child(div().id("edit-cancel").px_3().py_1().rounded_md().text_xs()
                    .text_color(t.muted_fg).cursor_pointer()
                    .hover(|style| style.bg(t.hover))
                    .on_click(cx.listener(|this, e, w, cx| this.close_edit(e, w, cx)))
                    .child("取消"))
                .child(div().id("edit-save").px_3().py_1().rounded_md().text_xs()
                    .text_color(Hsla { h: t.accent.h, s: t.accent.s, l: 0.15, a: 1.0 })
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .bg(t.accent)
                    .cursor_pointer()
                    .hover(|style| style.opacity(0.9))
                    .on_click(cx.listener(|this, e, w, cx| this.save_edit(e, w, cx)))
                    .child("保存")));

        overlay.child(panel).into_any_element()
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
        let position_ms = (cur_t * 1000.0) as u32;

        // 计算侧边栏列表可用高度（窗口高度 - topbar - 标题栏 - tabs）
        let window_height = w.viewport_size().height;
        let sidebar_list_height = (window_height - px(200.0)).max(px(200.0));

        // 侧边栏宽度响应式：按窗口宽度的 28% 计算，限制在 [240, 400] 之间。
        // 窗口拉大时侧边栏跟着变宽，歌名显示区域更多；拉小时最少 240px 不至于太挤。
        let sidebar_width = (w.viewport_size().width * 0.28)
            .max(px(240.0))
            .min(px(400.0));

        // 设置面板宽度响应式：按窗口宽度的 26% 计算，限制在 [260, 360] 之间。
        let settings_width = (w.viewport_size().width * 0.26)
            .max(px(260.0))
            .min(px(360.0));

        // 队列面板宽度响应式：比设置面板略窄（22%，[240, 320]）
        let queue_width = (w.viewport_size().width * 0.22)
            .max(px(240.0))
            .min(px(320.0));
        
        // ── 搜索输入框懒初始化 ──
        // InputState::new 需要窗口句柄，故推迟到 render 首次拿到 &mut Window 时创建。
        // cx.new 的闭包只接收 &mut Context<InputState>，window 从外部捕获。
        if self.search_input.is_none() {
            let input = cx.new(|cx2| {
                let mut s = InputState::new(w, cx2);
                // 搜索栏占位提示
                s.set_placeholder("搜索歌曲、歌手...", w, cx2);
                // 按 Esc 清空搜索
                s = s.clean_on_escape();
                s
            });
            // 订阅文本变化：把最新值同步到 search_query 并触发重绘
            let sub = cx.subscribe(&input, {
                let inp = input.clone();
                move |this, _entity, event, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.search_query = inp.read(cx).value().to_string();
                        cx.notify();
                    }
                }
            });
            self.search_input = Some(input);
            self.search_sub = Some(sub);
        }

        // 构建各模块
        let sidebar = ui::sidebar::build_sidebar(
            &self.folders,
            &self.loading_folders,
            &t,
            sidebar_list_height,
            self.search_input.as_ref().unwrap(),
            &self.search_query,
            &self.active_tab,
            &self.expanded_albums,
            &self.expanded_artists,
            &self.recent,
            self.album_grid,
            sidebar_width,
            cx,
        );
        let topbar = ui::topbar::build_topbar(sidebar_open, settings_open, &t, cx);
        
        // 歌词数据（传递给 center）
        let lyrics_data = self.lyrics.as_ref();
        let lyric_line_idx = self.lyric_line;
        
        let center = ui::center::build_center(
            &title, &artist, &album, &t, cx, lyrics_data, lyric_line_idx, position_ms,
            self.current_cover.as_ref(), self.lyric_font_step, self.lyric_centered,
        );
        // 进度条边界共享槽：player.rs 里的 canvas 在 paint 阶段写入真实矩形，
        // 点击/拖拽 seek 时再据此把窗口坐标换算成 0~1 比例（只在本帧内共享即可）。
        let track_bounds: Arc<Mutex<Option<Bounds<Pixels>>>> = Arc::new(Mutex::new(None));
        let player_bar = ui::player::build_player_bar(
            playing, cur_t, tot_t, self.shuffle, self.queue_open, track_bounds, &t, cx,
        );
        let settings_panel = ui::settings::build_settings(
            opacity, opacity_enabled, &self.slider,
            &t, theme_mode,
            &self.bg_slider, self.bg_image_blur,
            self.shuffle, self.repeat, &self.volume_slider, self.volume as f32,
            self.assoc_msg.clone(),
            settings_width,
            cx,
        );
        // 播放队列面板（第三组 UI）：展开时插在中间区和设置面板之间
        let queue_panel = if self.queue_open {
            let upcoming = self.upcoming_list(30);
            let cur_title = self.current
                .and_then(|(fi, ti)| self.folders.get(fi).and_then(|f| f.tracks.get(ti)))
                .map(|tr| tr.title.clone()).unwrap_or_else(|| "未播放".to_string());
            let cur_artist = self.current
                .and_then(|(fi, ti)| self.folders.get(fi).and_then(|f| f.tracks.get(ti)))
                .map(|tr| tr.artist.clone()).unwrap_or_default();
            Some(ui::queue::build_queue_panel(
                playing, self.eq_phase, &cur_title, &cur_artist, &upcoming,
                &t, queue_width, cx,
            ))
        } else {
            None
        };

        // ── 窗口视觉分层总览（透明 / 背景图）──
        // 从下到上共两层：
        //   1) bg_layer（绝对定位铺满）：背景图本身，可选再叠一层主题色 0.4 的"磨砂"遮罩。
        //   2) content（真正的 UI：topbar + body + 设置面板）。
        // 两个滑块分别控制这两层的"可见度"：
        //   · 窗口透明度开关 + "不透明度"滑块(opacity, 0.1~1.0)：
        //       关 → bg_layer 的图 opacity=1.0，content 用不透明 surface 盖住；
        //       开 → bg_layer 的图 opacity=opacity，无图时 content.bg(surface).opacity(opacity)，
        //             整个窗口半透明，桌面直接透出来。
        //   · "背景图透明度"滑块(bg_content_opacity, 0~1.0，仅选了背景图才出现)：
        //       content 的 surface 背景 alpha = 1.0 - 该值，值越大 content 越透、
        //       下层的背景图越明显；它只决定"墙纸在面板后露多少"，不会让窗口透到桌面。
        //   · 磨砂效果开关：在 bg_layer 上叠一层主题色 a:0.4 的 div，做出毛玻璃感。
        // 关键区分：WindowOptions.window_background=Transparent 是"窗口能否看穿桌面"的前提，
        // 与背景图无关；背景图是否可见由上面两层叠加结果决定。
        // 背景图的加载/解码见 pick_bg_image；持久化见 settings.json
        // (bg_image_path / bg_opacity / bg_blur / opacity / opacity_enabled)。
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
        // 播放队列面板放在设置面板左侧（两个都开时：中间区 | 队列 | 设置）
        let body = if let Some(panel) = queue_panel { body.child(panel) } else { body };

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
        // 迷你模式：整块内容换成紧凑单行布局（没有侧边栏 / 设置 / 大封面）
        let content = if self.mini_mode {
            let mini = ui::mini::build_mini_player(
                playing, cur_t, tot_t, &title, &artist,
                self.current_cover.as_ref(), &t, cx,
            );
            content.child(mini)
        } else {
            content.child(topbar).child(body)
        };

        // 根容器
        let root = div().id("root").size_full().relative();
        let root = root.child(bg_layer).child(content);

        // ── 编辑歌曲信息弹窗（模态遮罩 + 面板，绝对定位置于内容之上）──
        if self.edit_target.is_some() {
            root.child(self.build_edit_modal(w, cx))
        } else {
            root
        }
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
    // ── 单实例 + 命令行文件（第四组）──
    // 用法：Sonic.exe "D:\music\a.mp3" → 启动后直接播这首
    //      已有实例在跑 → 把路径交给它，自己立刻退出（避免开出一堆窗口）
    let cli_file = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    if !crate::tray::is_first_instance() {
        if let Some(path) = cli_file {
            crate::tray::write_pending_file(&path);
        }
        eprintln!("[main] 已有实例在运行，本次启动退出");
        return;
    }
    // 第一个实例：把自己的命令行文件也塞进同一个"待打开"通道，
    // 交给 new() 里那个每秒轮询统一处理（这样只有一条播放路径，不用写两份）。
    if let Some(path) = cli_file {
        crate::tray::write_pending_file(&path);
    }

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
                    // 窗口级透明：让整个窗口背景透明（Windows layered window 合成），
                    // 配合 titlebar.appears_transparent，桌面能透过窗口显示。
                    // 注意：这层只决定"窗口能不能看穿到桌面"，背景图是否可见由下方
                    // bg_layer/content 两层叠加 + 两个滑块控制，与这里无关。
                    window_background: WindowBackgroundAppearance::Transparent,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(100.0), px(100.0)),
                        size(px(1200.0), px(800.0)),
                    ))),
                    // 最小窗口尺寸：不允许缩到比初始尺寸还小（防止布局挤压）
                    window_min_size: Some(size(px(1200.0), px(800.0))),
                    ..Default::default()
                },
                |window, cx| {
                    // 主视图必须用 gpui_component::Root 包一层：Input 等控件在渲染时
                    // 会调用 Root::read，要求窗口根视图是 Root 类型，否则会 unwrap(None) panic。
                    let main_view = cx.new(|cx| MusicPlayer::new(cx));
                    cx.new(|cx| Root::new(main_view, window, cx))
                },
            );
        })
        .detach();
    });
}
