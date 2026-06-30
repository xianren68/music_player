// ─── 本地音乐播放器（gpui-component 版）───
mod models;
mod audio;
mod ui;

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{ActiveTheme as _, Root, slider::*};


use models::Folder;
use audio::Player;

/// 应用主结构
struct MusicPlayer {
    folders: Vec<Folder>,
    current: Option<(usize, usize)>,
    playing: bool,
    sidebar_open: bool,
    settings_open: bool,
    opacity: f32,
    opacity_enabled: bool,
    follow_system_theme: bool,
    progress: f64,
    player: Player,
    slider: Entity<SliderState>,
}

// 注册动作
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

        // 订阅滑块事件：更新透明度值
        let _subscription = cx.subscribe(&slider, |this, _, event: &SliderEvent, _cx| {
            match event {
                SliderEvent::Change(value) => {
                    this.opacity = value.start();
                }
            }
        });

        Self {
            folders: Vec::new(),
            current: None,
            playing: false,
            sidebar_open: false,
            settings_open: false,
            opacity: 1.0,
            opacity_enabled: false,
            follow_system_theme: true,
            progress: 0.0,
            player: Player::new(),
            slider,
        }
    }

    // ── 事件处理 ──

    /// 切换侧边栏显示
    fn toggle_sidebar(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    /// 切换设置面板显示
    fn toggle_settings(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = !self.settings_open;
        cx.notify();
    }

    /// 添加音乐文件夹
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

    /// 播放/暂停
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

    /// 下一曲
    fn next(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((fi, ti)) = self.current {
            if let Some((f, t)) = self.next_idx(fi, ti) {
                self.play(f, t);
                cx.notify();
            }
        }
    }

    /// 上一曲
    fn prev(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some((fi, ti)) = self.current {
            if let Some((f, t)) = self.prev_idx(fi, ti) {
                self.play(f, t);
                cx.notify();
            }
        }
    }

    /// 播放指定曲目
    fn play_at(&mut self, fi: usize, ti: usize, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.play(fi, ti);
        cx.notify();
    }

    /// 展开/折叠文件夹
    fn toggle_folder(&mut self, fi: usize, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.folders.get_mut(fi) {
            f.expanded = !f.expanded;
            cx.notify();
        }
    }

    /// 切换亮色主题
    fn set_light_theme(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.follow_system_theme = false;
        gpui_component::Theme::change(gpui_component::ThemeMode::Light, None, cx);
        cx.notify();
    }

    /// 切换暗色主题
    fn set_dark_theme(&mut self, _: &ClickEvent, _w: &mut Window, cx: &mut Context<Self>) {
        self.follow_system_theme = false;
        gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
        cx.notify();
    }

    // ── 播放控制 ──

    /// 获取第一首曲目
    fn first(&self) -> Option<(usize, usize)> {
        self.folders.iter().enumerate()
            .find(|(_, f)| !f.tracks.is_empty())
            .map(|(i, _)| (i, 0))
    }

    /// 获取下一首曲目索引
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

    /// 获取上一首曲目索引
    fn prev_idx(&self, fi: usize, ti: usize) -> Option<(usize, usize)> {
        if ti > 0 {
            return Some((fi, ti - 1));
        }
        if fi > 0 {
            let pf = fi - 1;
            if !self.folders[pf].tracks.is_empty() {
                return Some((pf, self.folders[pf].tracks.len() - 1));
            }
        }
        None
    }

    /// 播放指定位置的曲目
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
        let opacity = self.opacity;
        let opacity_enabled = self.opacity_enabled;
        let follow_system_theme = self.follow_system_theme;
        let is_dark = cx.theme().is_dark();

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
        let sidebar = ui::sidebar::build_sidebar(&self.folders, &self.current, cx);
        let topbar = ui::topbar::build_topbar(sidebar_open, cx);
        let center = ui::center::build_center(&title, &artist, &album, cx);
        let player_bar = ui::player::build_player_bar(playing, prog, cur_t, tot_t, cx);
        let settings_panel = ui::settings::build_settings(opacity, opacity_enabled, follow_system_theme, is_dark, &self.slider, cx);

        // 主区域
        let main = div().id("main").flex_1().relative()
            .when(!opacity_enabled, |this| this.bg(cx.theme().background))
            .child(topbar)
            .child(player_bar)
            .child(center);

        // 最终布局
        let root = div().id("root").size_full();
        let root = if opacity_enabled {
            root.opacity(opacity)
        } else {
            root.bg(cx.theme().background)
        };
        root.child(
            div().id("content").size_full().flex().flex_row()
                .when(sidebar_open, |this| this.child(sidebar))
                .child(main)
                .when(settings_open, |this| this.child(settings_panel))
        )
    }
}

// ── 入口 ──

fn main() {
    Application::new().run(move |cx| {
        // 初始化 gpui-component
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
                |window, cx| {
                    cx.new(move |cx| {
                        let view = cx.new(|cx| MusicPlayer::new(cx));
                        Root::new(view, window, cx)
                    })
                },
            );
        })
        .detach();
    });
}
