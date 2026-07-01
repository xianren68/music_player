// ─── 本地音乐播放器（gpui-component 版）───
mod models;
mod audio;
mod ui;
mod theme;

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{slider::*};
use theme::ThemeConfig;
use std::sync::Arc;

use models::Folder;
use audio::Player;

/// 主题模式
#[derive(Clone, PartialEq)]
enum ThemeMode { Dark, Light, Custom }

/// 应用主结构
struct MusicPlayer {
    folders: Vec<Folder>,
    current: Option<(usize, usize)>,
    playing: bool,
    sidebar_open: bool,
    settings_open: bool,
    opacity_enabled: bool,
    progress: f64,
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
}

actions!(music_player, [ToggleSidebar, ToggleSettings, AddFolder, PlayPause, Next, Prev]);

impl MusicPlayer {
    fn new(cx: &mut Context<Self>) -> Self {
        let slider = cx.new(|_| {
            SliderState::new()
                .min(0.1)
                .max(1.0)
                .step(0.01)
                .default_value(1.0)
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
                .default_value(1.0)
        });

        let bg_sub = cx.subscribe(&bg_slider, move |_, _, _event: &SliderEvent, cx| {
            cx.notify();
        });

        // 加载主题配置
        let theme = ThemeConfig::dark();

        Self {
            folders: Vec::new(),
            current: None,
            playing: false,
            sidebar_open: false,
            settings_open: false,
            opacity_enabled: false,
            progress: 0.0,
            player: Player::new(),
            slider,
            _subscription: subscription,
            bg_slider,
            _bg_sub: bg_sub,
            theme,
            theme_mode: ThemeMode::Dark,
            bg_image_path: None,
            bg_image_blur: false,
            bg_image: None,
        }
    }

    // ── 事件处理 ──

    fn toggle_sidebar(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    fn toggle_settings(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        cx.notify();
    }

    fn add_folder(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(dir) = rfd::FileDialog::new().set_title("选择音乐文件夹").pick_folder() {
            let tracks = audio::scan_dir(&dir);
            if !tracks.is_empty() {
                let name = dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                self.folders.push(Folder { name, expanded: true, tracks });
            }
            cx.notify();
        }
    }

    fn play_pause(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if self.current.is_none() {
            if let Some((fi, ti)) = self.first() {
                self.play(fi, ti);
            }
            return;
        }
        self.player.toggle(self.playing);
        self.playing = !self.playing;
        cx.notify();
    }

    fn next(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((fi, ti)) = self.current {
            if let Some((f, t)) = self.next_idx(fi, ti) {
                self.play(f, t);
                cx.notify();
            }
        }
    }

    fn prev(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((fi, ti)) = self.current {
            if let Some((f, t)) = self.prev_idx(fi, ti) {
                self.play(f, t);
                cx.notify();
            }
        }
    }

    fn play_at(&mut self, fi: usize, ti: usize, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.play(fi, ti);
        cx.notify();
    }

    fn toggle_folder(&mut self, fi: usize, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.folders.get_mut(fi) {
            f.expanded = !f.expanded;
            cx.notify();
        }
    }

    /// 应用暗色主题
    fn apply_dark(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.theme = ThemeConfig::dark();
        self.theme_mode = ThemeMode::Dark;
        cx.notify();
    }

    /// 应用亮色主题
    fn apply_light(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.theme = ThemeConfig::light();
        self.theme_mode = ThemeMode::Light;
        cx.notify();
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
                    let rgba = img.to_rgba8();
                    let frame = image::Frame::new(rgba);
                    self.bg_image = Some(Arc::new(RenderImage::new(
                        smallvec::SmallVec::from_elem(frame, 1)
                    )));
                }
            }
            cx.notify();
        }
    }

    /// 清除背景图片
    fn clear_bg_image(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.bg_image_path = None;
        self.theme.bg_image = None;
        self.bg_image = None;
        cx.notify();
    }

    /// 切换背景图磨砂效果（Switch 回调）
    fn toggle_bg_blur(&mut self, checked: &bool, _w: &mut Window, cx: &mut Context<Self>) {
        self.bg_image_blur = *checked;
        cx.notify();
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

    fn play(&mut self, fi: usize, ti: usize) {
        self.player.stop();
        if let Some(f) = self.folders.get(fi) {
            if let Some(t) = f.tracks.get(ti) {
                self.player.play(t);
                self.current = Some((fi, ti));
                self.playing = true;
                self.progress = 0.0;
            }
        }
    }
}

// ── UI 渲染 ──

impl Render for MusicPlayer {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_open = self.sidebar_open;
        let settings_open = self.settings_open;
        let opacity = self.slider.read(cx).value().start();
        let opacity_enabled = self.opacity_enabled;
        let t = self.theme.clone();
        let theme_mode = self.theme_mode.clone();

        // 获取当前曲目信息
        let (title, artist, album) = self.current
            .and_then(|(fi, ti)| self.folders.get(fi).and_then(|f| f.tracks.get(ti)))
            .map(|t| (t.title.clone(), t.artist.clone(), t.album.clone()))
            .unwrap_or_else(|| ("未播放".into(), "选择一首歌开始播放".into(), String::new()));

        let playing = self.playing;
        let (cur_t, tot_t) = self.current
            .and_then(|(fi, ti)| self.folders.get(fi).and_then(|f| f.tracks.get(ti)))
            .map(|t| (t.duration * self.progress, t.duration))
            .unwrap_or((0.0, 0.0));
        let prog = self.progress;

        // 构建各模块
        let sidebar = ui::sidebar::build_sidebar(&self.folders, &self.current, &t, cx);
        let topbar = ui::topbar::build_topbar(sidebar_open, settings_open, &t, cx);
        let center = ui::center::build_center(&title, &artist, &album, &t, cx);
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
            // 背景图透明度由窗口透明度控制
            let img_alpha = if opacity_enabled { opacity } else { 1.0 };
            // 磨砂：在图上叠加一层主题色
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

        // 主区域 —— 不设背景，透明
        let main = div().id("main").flex_1().relative()
            .child(topbar)
            .child(player_bar)
            .child(center);

        // 内容层
        let content = div().id("content").size_full().flex().flex_row();
        // 透明度控制：只改背景色的 alpha，不影响文字
        let content = if has_bg {
            // 有背景图：bg 用反转 alpha → 滑块越大图越不透明
            content.bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 1.0 - bg_content_opacity })
        } else if opacity_enabled {
            // 无背景图：整个 content 半透明透桌面
            content.bg(t.bg).opacity(opacity)
        } else {
            content.bg(t.bg)
        };
        let content = content
            .when(sidebar_open, |this| this.child(sidebar))
            .child(main)
            .when(settings_open, |this| this.child(settings_panel));

        // 根容器：透明，只做布局
        let root = div().id("root").size_full().relative();
        root.child(bg_layer).child(content)
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
