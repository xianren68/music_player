// ─── 侧边栏模块 ───
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::tooltip::Tooltip;
use gpui_component::input::{Input, InputState};
use crate::theme::ThemeConfig;
use crate::models::Folder;
use std::collections::BTreeMap;

/// 侧边栏当前展示的列表视图（只保留三个 tab）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ListView { Playlists, Album, Artist }

/// 分组类型（专辑/歌手分组的内部区分）
#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupKind { Album, Artist }

/// 扁平化后的列表项：侧边栏虚拟滚动统一用这一个枚举，避免按 tab 分多套渲染路径
#[derive(Clone)]
enum FlatItem {
    FolderHeader { fi: usize, name: String, count: usize, expanded: bool },
    GroupHeader { kind: GroupKind, key: String, name: String, sub: String, count: usize, expanded: bool },
    Track { fi: usize, ti: usize },
    Loading { name: String },
}

/// "播放列表"视图：按文件夹分组（文件夹头 + 展开后的歌曲），沿用旧版"全部"的样子
fn build_flat_playlists(folders: &[Folder]) -> Vec<FlatItem> {
    let mut items = Vec::new();
    for (fi, folder) in folders.iter().enumerate() {
        items.push(FlatItem::FolderHeader {
            fi,
            name: folder.name.clone(),
            count: folder.tracks.len(),
            expanded: folder.expanded,
        });
        if folder.expanded {
            for ti in 0..folder.tracks.len() {
                items.push(FlatItem::Track { fi, ti });
            }
        }
    }
    items
}

/// 专辑/歌手分组视图：按 key（专辑名/歌手名）聚合，展开后列出组内歌曲。
/// 用 BTreeMap 让分组名按字典序排列；空分组统一改名为"未知专辑"/"未知歌手"落在一起。
fn build_flat_group(folders: &[Folder], kind: GroupKind, expanded: &std::collections::HashSet<String>) -> Vec<FlatItem> {
    let mut map: BTreeMap<String, Vec<(usize, usize, String)>> = BTreeMap::new();
    for (fi, folder) in folders.iter().enumerate() {
        for (ti, track) in folder.tracks.iter().enumerate() {
            let key = match kind {
                GroupKind::Album => track.album.clone(),
                GroupKind::Artist => track.artist.clone(),
            };
            // 副标题：专辑视图显示组内第一首的歌手；歌手视图留空
            let sub = match kind {
                GroupKind::Album => track.artist.clone(),
                GroupKind::Artist => String::new(),
            };
            map.entry(key).or_default().push((fi, ti, sub));
        }
    }
    let mut items = Vec::new();
    for (key, entries) in map {
        let name = if key.is_empty() {
            match kind {
                GroupKind::Album => "未知专辑".to_string(),
                GroupKind::Artist => "未知歌手".to_string(),
            }
        } else {
            key.clone()
        };
        let is_expanded = expanded.contains(&key);
        let count = entries.len();
        let sub = entries.first().map(|e| e.2.clone()).unwrap_or_default();
        items.push(FlatItem::GroupHeader {
            kind,
            key,
            name,
            sub,
            count,
            expanded: is_expanded,
        });
        if is_expanded {
            for (fi, ti, _) in entries {
                items.push(FlatItem::Track { fi, ti });
            }
        }
    }
    items
}

/// 单个标签页按钮（分段控件的一个等宽段；选中态用强调色高亮）
fn build_tab(
    id: &'static str,
    label: &str,
    tab: ListView,
    active: bool,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id(id)
        .flex_1().h(px(28.0))
        .flex().items_center().justify_center()
        .text_xs().font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer()
        .rounded_full()
        .when(active, |this| this
            .bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.20 })
            .text_color(t.fg))
        .when(!active, |this| this
            .text_color(t.muted_fg)
            .hover(|style| style
                .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.06 })
                .text_color(t.fg)))
        .on_click(cx.listener(move |this, e, w, cx| this.set_tab(tab, e, w, cx)))
        .child(label.to_string())
        .into_any_element()
}

pub fn build_sidebar(
    folders: &[Folder],
    loading_folders: &[SharedString],
    t: &ThemeConfig,
    list_height: gpui::Pixels,
    search_input: &Entity<InputState>,
    search_query: &str,
    active_tab: &ListView,
    expanded_albums: &std::collections::HashSet<String>,
    expanded_artists: &std::collections::HashSet<String>,
    sidebar_width: gpui::Pixels,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    // 搜索栏（真实输入框，绑定到 search_input 状态）
    let search_bar = div().flex().items_center().gap_2()
        .px_2().py_1().rounded_lg().mt_3()
        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 })
        .border_1().border_color(t.border)
        .child(Input::new(search_input)
            .prefix(svg().path("icons/search.svg").size_4().text_color(t.muted_fg))
            .cleanable(true)
            .bordered(false)
            .appearance(false));

    // 标签页：分段控件样式（等宽三段 + 一个圆角轨道），选中态用强调色填充
    let tabs = div().flex().gap_1().p_1()
        .rounded_full()
        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 })
        .child(build_tab("tab-playlists", "播放列表", ListView::Playlists, *active_tab == ListView::Playlists, t, cx))
        .child(build_tab("tab-album", "专辑", ListView::Album, *active_tab == ListView::Album, t, cx))
        .child(build_tab("tab-artist", "歌手", ListView::Artist, *active_tab == ListView::Artist, t, cx));

    // 搜索模式：扁平展示匹配结果；否则按当前 tab 构建对应视图
    let query = search_query.trim();
    let is_search = !query.is_empty();
    let matched: Vec<(usize, usize)> = if is_search {
        // 大小写不敏感地匹配 歌名 / 歌手 / 专辑
        let ql: String = query.to_lowercase();
        let mut v = Vec::new();
        for (fi, folder) in folders.iter().enumerate() {
            for (ti, track) in folder.tracks.iter().enumerate() {
                if track.title.to_lowercase().contains(&ql)
                    || track.artist.to_lowercase().contains(&ql)
                    || track.album.to_lowercase().contains(&ql)
                {
                    v.push((fi, ti));
                }
            }
        }
        v
    } else {
        Vec::new()
    };

    // 扁平化列表：搜索用匹配结果；否则按 tab 选择分组方式
    let mut flat: Vec<FlatItem> = if is_search {
        matched.iter().map(|&(fi, ti)| FlatItem::Track { fi, ti }).collect()
    } else {
        match *active_tab {
            ListView::Playlists => build_flat_playlists(folders),
            ListView::Album => build_flat_group(folders, GroupKind::Album, expanded_albums),
            ListView::Artist => build_flat_group(folders, GroupKind::Artist, expanded_artists),
        }
    };
    // 非搜索时，把"正在扫描"的文件夹以 Loading 项置顶展示（各视图通用）
    if !is_search {
        let mut loading: Vec<FlatItem> = loading_folders.iter().map(|lf| {
            let dir = std::path::Path::new(lf.as_ref());
            let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("音乐").to_string();
            FlatItem::Loading { name }
        }).collect();
        loading.extend(flat);
        flat = loading;
    }
    let total = flat.len();
    let is_empty = flat.is_empty();
    let empty_msg = if is_search {
        "未找到匹配的歌曲"
    } else if folders.is_empty() {
        "在设置中添加音乐文件夹"
    } else {
        "暂无内容"
    };

    // 列表项行高：卡片 56 + 底部 6px 间距 = 62。
    // 间距必须做进元素高度（外层容器 62、内层卡片 56），不能用 margin —— uniform_list
    // 按"索引 × 测量高度"绝对定位，margin 会被忽略导致选中行底色贴在一起。
    let row_h = px(62.0);

    // ── 虚拟滚动列表 ──
    let list = if is_empty {
        div().text_color(t.muted_fg).text_center().py_12().text_sm()
            .child(empty_msg)
            .into_any()
    } else {
        let t_clone = t.clone();
        let flat_clone = flat.clone();

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
                    let item = flat_clone.get(ix).cloned();
                    if let Some(item) = item {
                        // 封面懒加载防抖：歌曲出现在可视区域且没有封面缓存时，加入待处理队列
                        if let FlatItem::Track { fi, ti } = &item {
                            if let Some(track) = this.folders.get(*fi).and_then(|f| f.tracks.get(*ti)) {
                                if track.cover_path.is_none() {
                                    let key = track.path.to_string_lossy().to_string();
                                    let in_pending = this.pending_cover_loads.iter()
                                        .any(|(_, _, _, k)| k == &key);
                                    if !this.loading_covers.contains(&key) && !in_pending {
                                        needs_cover.push((*fi, *ti, track.path.clone(), key));
                                    }
                                }
                            }
                        }
                        // 渲染卡片（h=56），再用 62 高的外壳包一层 → 底部 6px 实打实的间距
                        let card = render_list_item(
                            item, ix, &this.folders, this.current, this.hovered_track, &t_clone,
                            bar_h1, bar_h2, bar_h3, cx,
                        );
                        items.push(
                            div().id(("row", ix as u64))
                                .w_full().h(row_h).flex().flex_col()
                                .child(card)
                                .into_any_element(),
                        );
                    }
                }

                // 有新封面需要加载 → 加入 pending 并启动/重置防抖定时器
                if !needs_cover.is_empty() {
                    this.pending_cover_loads.extend(needs_cover);
                    this.cover_debounce_gen = this.cover_debounce_gen.wrapping_add(1);
                    let expected_gen = this.cover_debounce_gen;
                    cx.spawn(async move |this, cx| {
                        // 防抖延迟 800ms：这段时间内如果又滚动了，gen 会变，老的定时器自动丢弃
                        cx.background_spawn(async {
                            std::thread::sleep(std::time::Duration::from_millis(800));
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

    // 侧边栏宽度随窗口大小响应式（由调用方按窗口宽度算好传入）
    div().id("sidebar").flex_none().w(sidebar_width).h_full().flex_col()
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
        // 标签页（分段控件）
        .child(div().px_3().pb_2().child(tabs))
        // 歌曲列表（虚拟滚动）— 容器有 px_3 pb_3
        .child(div().id("sidebar-list").flex_1().min_h_0().overflow_hidden()
            .px_3().pb_3()
            .child(list))
        .into_any()
}

/// 渲染单个列表项（卡片本体，高 56；间距由调用方用 62 外壳包出）
fn render_list_item(
    item: FlatItem,
    ix: usize,
    folders: &[Folder],
    current: Option<(usize, usize)>,
    hovered_track: Option<(usize, usize)>,
    t: &ThemeConfig,
    bar_h1: f32,
    bar_h2: f32,
    bar_h3: f32,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    match item {
        FlatItem::FolderHeader { fi, name, count, expanded } => {
            let fidx = fi;
            div().id(("fh", ix as u64))
                .w_full().h(px(56.0))
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
        FlatItem::GroupHeader { kind, key, name, sub, count, expanded } => {
            div().id(("gh", ix as u64))
                .w_full().h(px(56.0))
                .flex().items_center().justify_between()
                .px_3().rounded_md()
                // 已展开的分组用淡强调色底，表示"当前打开/选中"
                .when(expanded, |this| this.bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.10 }))
                .hover(|style| style.bg(t.hover))
                .cursor_pointer()
                // 点击展开/收起该分组（各自维护展开集合，切回 tab 后仍保持）
                .on_click(cx.listener(move |this, _e, _w, cx| {
                    match kind {
                        GroupKind::Album => {
                            if this.expanded_albums.contains(&key) {
                                this.expanded_albums.remove(&key);
                            } else {
                                this.expanded_albums.insert(key.clone());
                            }
                        }
                        GroupKind::Artist => {
                            if this.expanded_artists.contains(&key) {
                                this.expanded_artists.remove(&key);
                            } else {
                                this.expanded_artists.insert(key.clone());
                            }
                        }
                    }
                    cx.notify();
                }))
                .child(div().flex().items_center().gap_2()
                    .child(div().text_color(if expanded { t.accent_light } else { t.muted_fg }).text_xs()
                        .child(if expanded { "▼" } else { "▶" }))
                    .child(div().flex_col().gap(px(1.0))
                        .child(div().text_color(t.fg).text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(name.to_string()))
                        .when(!sub.is_empty(), |this| this
                            .child(div().text_color(t.muted_fg).text_xs().child(sub.to_string())))))
                .child(div().text_color(t.muted_fg).text_xs()
                    .child(format!("{}", count)))
                .into_any_element()
        }
        FlatItem::Track { fi, ti } => {
            let track = match folders.get(fi).and_then(|f| f.tracks.get(ti)) {
                Some(t) => t,
                None => return div().into_any_element(),
            };
            let cur = current == Some((fi, ti));
            let dur = crate::audio::fmt_time(track.duration);
            let title_str = track.title.clone();
            let artist_str = track.artist.clone();
            // 本行是否悬停：由调用方传入的 hovered_track 状态驱动 ⋯ 按钮展开。
            // （不在渲染函数里 read view 状态——渲染期间 view 已被 &mut 借用，
            //   read 会触发借用冲突 panic；由 processor 闭包用 this 直接读好传进来）
            let is_hovered = hovered_track == Some((fi, ti));
            let btn_w = if is_hovered { px(26.0) } else { px(0.0) };
            let btn_opacity = if is_hovered { 1.0 } else { 0.0 };

            div().id(("t", ix as u64))
                .h(px(56.0))
                .flex().items_center().gap_3()
                .ml_2().mr_2().px_3().rounded_md()
                .when(cur, |this| this.bg(t.active))
                .hover(|style| style.bg(if cur { t.active } else { t.hover }))
                .cursor_pointer()
                // 更新悬停状态（进入/离开本行），驱动 ⋯ 按钮展开/收起。
                // 离开时只清自己的状态，避免鼠标从 A 移到 B 时 A 的 leave 误清 B
                .on_hover(cx.listener(move |this, hovered: &bool, _w, cx| {
                    if *hovered {
                        if this.hovered_track != Some((fi, ti)) {
                            this.hovered_track = Some((fi, ti));
                            cx.notify();
                        }
                    } else if this.hovered_track == Some((fi, ti)) {
                        this.hovered_track = None;
                        cx.notify();
                    }
                }))
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
                // 时长 + ⋯ 编辑按钮（同一子容器，内部无 gap）：
                // 按钮常驻 flex 流（元素树不变），由 hovered_track 状态驱动
                // 宽度 0↔26px + 透明度 0↔1（notify 强制重排，可靠展开/收起）。
                // 平时不占位（行尾就是时长）；悬停时展开把时长挤到左侧。
                .child(div().flex().items_center()
                    .child(div().flex_none().pr_3().text_xs().text_color(t.muted_fg).child(dur))
                    .child(div().id(("te", ix as u64))
                        .flex_none().w(btn_w).overflow_hidden()
                        .h(px(24.0))
                        .rounded(px(6.0))
                        .border_1().border_color(t.border)
                        .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.04 })
                        .text_color(t.muted_fg)
                        .opacity(btn_opacity)
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.1 }))
                        .on_click(cx.listener(move |this, _e, w, cx| {
                            cx.stop_propagation();
                            this.open_edit(fi, ti, w, cx);
                        }))
                        .child(div().flex().items_center().justify_center().size_full()
                            .child(svg().path("icons/more.svg").size_4().text_color(t.muted_fg)))))
                .into_any_element()
        }
        FlatItem::Loading { name } => {
            div().id(("loading", ix as u64))
                .w_full().h(px(56.0))
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
