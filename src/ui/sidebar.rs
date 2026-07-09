// ─── 侧边栏模块 ───
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::tooltip::Tooltip;
use crate::theme::ThemeConfig;
use crate::models::Folder;

/// 列表项的引用（用于在 uniform_list 闭包内渲染）
enum ListItemRef<'a> {
    FolderHeader { fi: usize, name: &'a str, count: usize, expanded: bool },
    Track { fi: usize, ti: usize, track: &'a crate::models::Track },
    Loading { name: &'a str },
}

/// 根据 ix 在 folders + loading_folders 中查找对应的列表项
fn find_list_item<'a>(
    folders: &'a [Folder],
    loading_folders: &'a [SharedString],
    ix: usize,
) -> Option<ListItemRef<'a>> {
    let mut idx = 0;
    for (fi, folder) in folders.iter().enumerate() {
        if idx == ix {
            return Some(ListItemRef::FolderHeader {
                fi,
                name: &folder.name,
                count: folder.tracks.len(),
                expanded: folder.expanded,
            });
        }
        idx += 1;
        if folder.expanded {
            let len = folder.tracks.len();
            if ix < idx + len {
                let ti = ix - idx;
                return Some(ListItemRef::Track { fi, ti, track: &folder.tracks[ti] });
            }
            idx += len;
        }
    }
    let loading_idx = ix.saturating_sub(idx);
    if loading_idx < loading_folders.len() {
        let path_str = &loading_folders[loading_idx];
        let dir = std::path::Path::new(path_str.as_ref());
        let name = dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("音乐");
        Some(ListItemRef::Loading { name })
    } else {
        None
    }
}

pub fn build_sidebar(
    folders: &[Folder],
    loading_folders: &[SharedString],
    t: &ThemeConfig,
    list_height: gpui::Pixels,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    // 搜索栏
    let search_bar = div().flex().items_center().gap_2()
        .px_3().py_2().rounded_lg().mt_3()
        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 })
        .border_1().border_color(t.border)
        .child(svg().path("icons/search.svg").size_4().text_color(t.muted_fg))
        .child(div().text_sm().text_color(t.muted_fg).child("搜索歌曲、歌手..."));

    // 标签页
    let tabs = div().flex().gap_2().px_1()
        .child(
            div().px_3().py_1().rounded_full().text_xs().font_weight(gpui::FontWeight::MEDIUM)
                .bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.15 })
                .text_color(t.fg).child("全部")
        )
        .child(
            div().px_3().py_1().rounded_full().text_xs().font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.muted_fg).child("播放列表")
        )
        .child(
            div().px_3().py_1().rounded_full().text_xs().font_weight(gpui::FontWeight::MEDIUM)
                .text_color(t.muted_fg).child("专辑")
        );

    // 计算总列表项数
    let total: usize = {
        let mut count = 0;
        for folder in folders {
            count += 1;
            if folder.expanded {
                count += folder.tracks.len();
            }
        }
        count += loading_folders.len();
        count
    };

    let has_folders = !folders.is_empty();
    let has_loading = !loading_folders.is_empty();
    let is_empty = !has_folders && !has_loading;

    // ── 虚拟滚动列表 ──
    let list = if is_empty {
        div().text_color(t.muted_fg).text_center().py_12().text_sm()
            .child("在设置中添加音乐文件夹")
            .into_any()
    } else {
        let t_clone = t.clone();

        uniform_list(
            "sidebar-virtual-list",
            total,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                let phase = this.eq_phase as f32 * 0.6;
                let bar_h1 = 4.0 + 8.0 * (phase + 0.0).sin().abs();
                let bar_h2 = 4.0 + 10.0 * (phase + 1.2).sin().abs();
                let bar_h3 = 4.0 + 8.0 * (phase + 2.4).sin().abs();

                let mut items: Vec<AnyElement> = Vec::with_capacity(range.end - range.start);
                // 防抖收集：本轮渲染中需要加载封面的歌曲
                let mut needs_cover: Vec<(usize, usize, std::path::PathBuf, String)> = Vec::new();
                for ix in range {
                    let item = find_list_item(&this.folders, &this.loading_folders, ix);
                    if let Some(item) = item {
                        // 封面懒加载防抖：歌曲出现在可视区域且没有封面缓存时，加入待处理队列
                        if let ListItemRef::Track { fi, ti, track } = &item {
                            if track.cover_path.is_none() {
                                let key = track.path.to_string_lossy().to_string();
                                let in_pending = this.pending_cover_loads.iter()
                                    .any(|(_, _, _, k)| k == &key);
                                if !this.loading_covers.contains(&key) && !in_pending {
                                    needs_cover.push((*fi, *ti, track.path.clone(), key));
                                }
                            }
                        }
                        items.push(render_list_item(
                            item, ix, this.current, &t_clone,
                            bar_h1, bar_h2, bar_h3, cx,
                        ));
                    }
                }

                // 有新封面需要加载 → 加入 pending 并启动/重置防抖定时器
                if !needs_cover.is_empty() {
                    this.pending_cover_loads.extend(needs_cover);
                    this.cover_debounce_gen = this.cover_debounce_gen.wrapping_add(1);
                    let expected_gen = this.cover_debounce_gen;
                    cx.spawn(async move |this, cx| {
                        // 防抖延迟 300ms：这段时间内如果又滚动了，gen 会变，老的定时器自动丢弃
                        cx.background_spawn(async {
                            std::thread::sleep(std::time::Duration::from_millis(300));
                        }).await;
                        this.update(cx, |this, cx| {
                            // 代数不匹配 → 中间有新渲染 → 丢弃本轮，让新的定时器处理
                            if this.cover_debounce_gen != expected_gen {
                                return;
                            }
                            // 防抖到点，批量处理所有待加载封面
                            for (fi, ti, path, key) in std::mem::take(&mut this.pending_cover_loads) {
                                // 安全检查：文件夹/歌曲索引可能已变化
                                if fi >= this.folders.len() { continue; }
                                if ti >= this.folders[fi].tracks.len() { continue; }
                                if this.folders[fi].tracks[ti].path != path { continue; }
                                this.loading_covers.insert(key.clone());
                                let fi2 = fi;
                                let ti2 = ti;
                                cx.spawn(async move |this, cx| {
                                    let cover_path = cx.background_spawn(async move {
                                        crate::audio::extract_and_cache_cover(&path)
                                    }).await;
                                    this.update(cx, |this, cx| {
                                        this.loading_covers.remove(&key);
                                        if let Some(folder) = this.folders.get_mut(fi2) {
                                            if let Some(track) = folder.tracks.get_mut(ti2) {
                                                if track.cover_path.is_none() {
                                                    track.cover_path = cover_path;
                                                    this.save_settings_for_cache();
                                                }
                                            }
                                        }
                                        cx.notify();
                                    }).ok();
                                }).detach();
                            }
                        }).ok();
                    }).detach();
                }
                items
            }),
        )
        .h(list_height)
        .into_any()
    };

    div().id("sidebar").flex_none().w(px(280.0)).h_full().flex_col()
        .bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 0.5 })
        .border_r_1().border_color(t.border)
        // 标题栏
        .child(
            div().px_4().pt_5().pb_4().flex_col().gap_5()
                .child(div().flex().items_center().gap_2()
                    .child(svg().path("icons/music.svg").size_5().text_color(t.accent_light))
                    .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).text_color(t.fg).child("音乐库")))
                .child(search_bar)
        )
        // 标签页
        .child(div().px_3().pb_2().child(tabs))
        // 歌曲列表（虚拟滚动）— 和旧版一样：容器有 px_3 pb_3
        .child(div().id("sidebar-list").flex_1().min_h_0().overflow_hidden()
            .px_3().pb_3()
            .child(list))
        .into_any()
}

/// 渲染单个列表项（完全按照旧版样式）
fn render_list_item(
    item: ListItemRef,
    ix: usize,
    current: Option<(usize, usize)>,
    t: &ThemeConfig,
    bar_h1: f32,
    bar_h2: f32,
    bar_h3: f32,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    match item {
        ListItemRef::FolderHeader { fi, name, count, expanded } => {
            let fidx = fi;
            div().id(("fh", ix as u64))
                .w_full()
                .h(px(56.0))  // 统一高度：和歌曲项一样
                .flex().items_center().justify_between()
                .px_3().rounded_md()
                .hover(|style| style.bg(t.hover))
                .cursor_pointer()
                .on_click(cx.listener(move |this, e, w, cx| this.toggle_folder(fidx, e, w, cx)))
                .child(div().flex().items_center().gap_2()
                    .child(div().text_color(t.accent_light).text_xs()
                        .child(if expanded { "▼" } else { "▶" }))
                    .child(div().text_color(t.fg).text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(name.to_string())))
                .child(div().text_color(t.muted_fg).text_xs()
                    .child(format!("{}", count)))
                .into_any_element()
        }
        ListItemRef::Track { fi, ti, track } => {
            let cur = current == Some((fi, ti));
            let dur = crate::audio::fmt_time(track.duration);
            let title_str = track.title.clone();
            let artist_str = track.artist.clone();

            div().id(("t", ix as u64))
                .w_full()
                .h(px(56.0))
                .flex().items_center().gap_3()
                .ml_2().px_3().rounded_md()
                .when(cur, |this| this.bg(t.active))
                .hover(|style| style.bg(if cur { t.active } else { t.hover }))
                .cursor_pointer()
                .on_click(cx.listener(move |this, e, w, cx| this.play_at(fi, ti, e, w, cx)))
                // tooltip 显示完整歌名（只在作者非空时显示 "歌名 - 作者"）
                .tooltip(move |_window, cx| {
                    let text = if artist_str.is_empty() {
                        title_str.clone()
                    } else {
                        format!("{} - {}", title_str, artist_str)
                    };
                    cx.new(|_| Tooltip::new(text)).into()
                })
                // 序号 / 播放动画（固定宽度）
                .child(if cur {
                    div().flex_none().w(px(20.0)).h(px(18.0))
                        .flex().items_end().justify_center()
                        .gap(px(2.0))
                        .child(div().w(px(3.0)).h(px(bar_h1))
                            .rounded(px(1.0)).bg(t.accent_light))
                        .child(div().w(px(3.0)).h(px(bar_h2))
                            .rounded(px(1.0)).bg(t.accent_light))
                        .child(div().w(px(3.0)).h(px(bar_h3))
                            .rounded(px(1.0)).bg(t.accent_light))
                        .into_any_element()
                } else {
                    div().flex_none().w(px(24.0)).text_xs().text_color(t.muted_fg)
                        .text_center().child(format!("{}", ti + 1))
                        .into_any_element()
                })
                // 封面（从缓存加载，没有则显示占位符）
                .child(render_track_cover(track, t))
                // 歌曲信息（flex_1 自动收缩）
                .child(div().flex_1().flex_col().overflow_hidden()
                    .justify_center()
                    .gap(px(1.0))
                    .child(div().text_sm().text_ellipsis().whitespace_nowrap()
                        .text_color(if cur { t.accent_light } else { t.fg })
                        .child(track.title.clone()))
                    .child(div().text_xs().text_ellipsis().whitespace_nowrap()
                        .text_color(t.muted_fg).child(track.artist.clone())))
                // 时长（固定宽度）
                .child(div().flex_none().pr_3().text_xs().text_color(t.muted_fg).child(dur))
                .into_any_element()
        }
        ListItemRef::Loading { name } => {
            div().id(("loading", ix as u64))
                .w_full()
                .h(px(56.0))  // 统一高度
                .flex().items_center().justify_between()
                .px_3().rounded_md()
                .child(div().flex().items_center().gap_2()
                    .child(div().text_color(t.accent_light).text_xs().child("⏳"))
                    .child(div().text_color(t.fg).text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(name.to_string())))
                .child(div().text_color(t.muted_fg).text_xs().child("加载中..."))
                .into_any_element()
        }
    }
}

/// 渲染歌曲封面（从缓存加载，没有则显示占位符）
fn render_track_cover(track: &crate::models::Track, t: &ThemeConfig) -> AnyElement {
    // 如果有缓存的封面路径，尝试加载
    if let Some(ref cover_path) = track.cover_path {
        if let Ok(cover_data) = std::fs::read(cover_path) {
            if let Ok(cover_img) = image::load_from_memory(&cover_data) {
                let mut rgba = cover_img.to_rgba8();
                // GPUI 的 RenderImage 期望 BGRA 格式，需要交换 R/B 通道
                for pixel in rgba.pixels_mut() {
                    pixel.0.swap(0, 2);
                }
                let frame = image::Frame::new(rgba);
                let render_img = std::sync::Arc::new(RenderImage::new(
                    smallvec::SmallVec::from_elem(frame, 1)
                ));
                return div().flex_none().w(px(40.0)).h(px(40.0)).rounded_md()
                    .overflow_hidden()
                    .child(gpui::img(render_img)
                        .w_full().h_full()
                        .object_fit(gpui::ObjectFit::Cover))
                    .into_any_element();
            }
        }
    }
    // 默认占位符
    div().flex_none().w(px(40.0)).h(px(40.0)).rounded_md()
        .bg(t.surface).flex().items_center().justify_center()
        .text_xs().text_color(t.muted_fg).child("♪")
        .into_any_element()
}
