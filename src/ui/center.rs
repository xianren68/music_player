// ─── 中间内容模块 ───
use gpui::*;
// when / when_some 这类条件构造方法来自 FluentBuilder
use gpui::prelude::FluentBuilder;
use std::sync::Arc;
use crate::theme::ThemeConfig;
use crate::lyrics::Lyrics;

/// 估算文本渲染宽度（用于判断歌名是否超出容器）
/// font_size: 字体像素大小（text_2xl ≈ 24px）
fn estimate_text_width(text: &str, font_size: f32) -> f32 {
    text.chars().map(|c| {
        if c.is_ascii() {
            font_size * 0.6  // ASCII 字符约为半宽（加粗略宽）
        } else {
            font_size * 1.05 // CJK 等宽字符为全宽（加粗略宽）
        }
    }).sum()
}

pub fn build_center(
    title: &str,
    artist: &str,
    _album: &str,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
    lyrics: Option<&Lyrics>,
    lyric_line: Option<usize>,
    position_ms: u32,
    cover: Option<&Arc<RenderImage>>,
    // ── 歌词增强（第三组 UI 设计）：字号档位 -1~2、是否居中 ──
    lyric_font_step: i32,
    lyric_centered: bool,
) -> AnyElement {
    // 字号缩放系数：档位每加一档放大 15%，最低档缩到 85%
    let scale = match lyric_font_step {
        s if s <= -1 => 0.85,
        0 => 1.0,
        1 => 1.15,
        _ => 1.3,
    };
    // 歌词区高度跟着字号走，避免放大后字幕被裁掉
    let lyric_height = px(120.0 + (lyric_font_step.max(0) as f32) * 26.0);

    // 估算歌名宽度，超出 400px 才启用滚动
    let title_w = estimate_text_width(title, 24.0); // text_2xl ≈ 24px
    let should_scroll = title_w > 400.0;

    div().id("center")
        .flex().flex_col().items_center()
        // 封面 + 歌曲信息 + 歌词 整体垂直居中
        .justify_center()
        .gap_4()
        // ── 专辑封面 ──
        .child(
            div().id("art").flex_none().w(px(200.0)).h(px(200.0)).rounded_2xl()
                .bg(t.surface).shadow_2xl()
                .child(if let Some(cover) = cover {
                    // 显示专辑封面
                    img(cover.clone())
                        .w_full().h_full()
                        .object_fit(gpui::ObjectFit::Cover)
                        .rounded_2xl()
                        .into_any_element()
                } else {
                    // 默认图标（需要 w_full h_full 才能居中）
                    div().w_full().h_full()
                        .flex().items_center().justify_center()
                        .text_size(px(64.0)).text_color(t.muted_fg).child("♪")
                        .into_any_element()
                })
        )
        // ── 曲目信息 ──
        .child(
            div().flex_col().items_center().text_center()
                // 歌名：超出容器宽度时才循环滚动
                .child(
                    div().w(px(400.0)).overflow_hidden()
                        .child({
                            let base = div().text_2xl().font_weight(gpui::FontWeight::BOLD)
                                .text_color(t.fg)
                                .whitespace_nowrap()
                                .child(title.to_string());
                            if should_scroll {
                                base.with_animation(
                                    "title-scroll",
                                    Animation::new(std::time::Duration::from_secs(12))
                                        .repeat()
                                        .with_easing(|t| t), // 线性
                                    |this, delta| {
                                        // delta 从 0 到 1，从右滚到左
                                        this.relative().left(px((1.0 - delta) * 400.0 - 50.0))
                                    }
                                ).into_any_element()
                            } else {
                                base.into_any_element()
                            }
                        })
                )
                .child(div().text_base().text_color(t.muted).mt_1()
                    .child(artist.to_string()))
        )
        // ── 歌词显示区域 ──
        // 字号按档位缩放；对齐由「居中 / 左对齐」控制条切换（左对齐时整体靠左，方便配合封面看的布局）
        .child(
            div().id("lyrics-area").flex_none()
                .w(px(400.0)).h(lyric_height)
                .overflow_hidden()
                .flex().flex_col().justify_center()
                .when(lyric_centered, |this| this.items_center())
                .when(!lyric_centered, |this| this.items_start())
                .child(if let Some(ref lyrics) = lyrics {
                    if let Some(line_idx) = lyric_line {
                        // 显示当前歌词行及其上下文（前后各2行）
                        let start = line_idx.saturating_sub(2);
                        let end = (line_idx + 3).min(lyrics.lines.len());

                        div().flex_col().gap_2()
                            .when(lyric_centered, |this| this.items_center())
                            .when(!lyric_centered, |this| this.items_start())
                            .children((start..end).map(|i| {
                                let line = &lyrics.lines[i];
                                let is_current = i == line_idx;
                                // 内联小工具：按当前对齐方式给行加居中，省得每处都写一遍
                                let centerize = |el: Div| -> Div {
                                    if lyric_centered { el.text_center() } else { el }
                                };

                                // 当前行且支持逐字高亮：用逐字渲染
                                if is_current {
                                    if let Some(ref highlights) = lyrics.get_word_highlights(position_ms, i) {
                                        // 逐字高亮渲染
                                        let line_el = div().flex().flex_wrap()
                                            .items_center()
                                            .when(lyric_centered, |this| this.justify_center());
                                        let line_el = line_el.children(
                                            highlights.iter().map(|wh| {
                                                div()
                                                    .text_color(if wh.highlighted { t.accent_light } else { t.muted })
                                                    .child(wh.text.clone())
                                            })
                                        );
                                        return centerize(div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_size(px(16.0 * scale)))
                                            .child(line_el)
                                            .into_any();
                                    }
                                }

                                // 非当前行，或当前行但不支持逐字：整行渲染
                                let (color, weight, size) = if is_current {
                                    (t.accent_light, gpui::FontWeight::SEMIBOLD, px(16.0 * scale))
                                } else if i < line_idx {
                                    (t.muted_fg, gpui::FontWeight::NORMAL, px(13.0 * scale))
                                } else {
                                    (t.muted, gpui::FontWeight::NORMAL, px(13.0 * scale))
                                };

                                centerize(div()
                                    .text_color(color)
                                    .font_weight(weight)
                                    .text_size(size))
                                    .child(line.text.clone())
                                    .into_any()
                            }))
                            .into_any()
                    } else {
                        // 有歌词但未到第一句：显示前两行歌词（全暗色）
                        let preview_count = 2.min(lyrics.lines.len());
                        div().flex_col().gap_2()
                            .when(lyric_centered, |this| this.items_center())
                            .when(!lyric_centered, |this| this.items_start())
                            .children((0..preview_count).map(|i| {
                                let el = div()
                                    .text_color(t.muted)
                                    .font_weight(gpui::FontWeight::NORMAL)
                                    .text_size(px(13.0 * scale));
                                let el = if lyric_centered { el.text_center() } else { el };
                                el.child(lyrics.lines[i].text.clone())
                                    .into_any()
                            }))
                            .into_any()
                    }
                } else {
                    // 没有歌词
                    div().text_color(t.muted).text_sm()
                        .child("暂无歌词")
                        .into_any()
                })
        )
        // ── 歌词工具条（第三组 UI 设计）：字号 −/+、居中/左对齐，外加一个占位的"翻译"开关 ──
        .child(lyric_controls(t, lyric_font_step, lyric_centered, cx))
        .into_any()
}

/// 歌词工具条：一排小圆角按钮，平时半透明、hover 才实心，尽量不抢歌词的视觉。
/// 说明：字号/对齐是真接状态（只影响显示）；「翻译」暂未接数据源，做灰态占位。
fn lyric_controls(
    t: &ThemeConfig,
    font_step: i32,
    centered: bool,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    /// 点击回调别名：gpui 的 on_click 参数是裸闭包，抽成类型别名才能当函数参数传
    type ClickCb = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

    // 工具条按钮统一样式：icon 型（只放图标）
    let icon_btn = |id: &'static str, icon: &'static str, active: bool, on_click: ClickCb| {
        div().id(id).w(px(24.0)).h(px(24.0)).rounded_md()
            .flex().items_center().justify_center()
            .cursor_pointer()
            .when(active, |this| this.bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.18 }))
            .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }))
            .on_click(move |e, w, cx| on_click(e, w, cx))
            .child(svg().path(icon).size_3_5()
                .text_color(if active { t.accent_light } else { t.muted_fg }))
    };
    // 文本型（字号 −/+ 这种，用字符更省事也更清楚）
    let text_btn = |id: &'static str, label: &'static str, on_click: ClickCb| {
        div().id(id).w(px(24.0)).h(px(24.0)).rounded_md()
            .flex().items_center().justify_center()
            .cursor_pointer()
            .text_sm().text_color(t.muted_fg)
            .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }).text_color(t.fg))
            .on_click(move |e, w, cx| on_click(e, w, cx))
            .child(label)
    };

    div().id("lyric-ctl").flex().items_center().gap_1().mt_1()
        .opacity(0.75)
        // 字号
        .child(svg().path("icons/type.svg").size_3_5().text_color(t.muted_fg).mr_1())
        .child(text_btn("lyric-font-down", "−",
            Box::new(cx.listener(|this, _e, _w, cx| this.lyric_font_step_change(-1, cx)))))
        .child(div().min_w(px(28.0)).text_xs().text_color(t.muted_fg).text_center()
            .child(format!("{}档", font_step + 1)))
        .child(text_btn("lyric-font-up", "+",
            Box::new(cx.listener(|this, _e, _w, cx| this.lyric_font_step_change(1, cx)))))
        // 分隔线
        .child(div().w(px(1.0)).h(px(14.0)).mx_2().bg(t.border))
        // 对齐：居中 / 左对齐
        .child(icon_btn("lyric-align", "icons/align-center.svg", centered,
            Box::new(cx.listener(|this, _e, _w, cx| this.toggle_lyric_align(cx)))))
        // 分隔线
        .child(div().w(px(1.0)).h(px(14.0)).mx_2().bg(t.border))
        // 翻译：数据源待接，先做灰态占位（不可点）
        .child(
            div().id("lyric-translate").px_2().h(px(24.0)).rounded_md()
                .flex().items_center().gap_1()
                .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.04 })
                .child(svg().path("icons/globe.svg").size_3_5().text_color(t.muted_fg))
                .child(div().text_xs().text_color(t.muted_fg).child("翻译"))
                .opacity(0.45)
        )
        .into_any()
}
