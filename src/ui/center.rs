// ─── 中间内容模块 ───
use gpui::*;
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
    _cx: &mut Context<crate::MusicPlayer>,
    lyrics: Option<&Lyrics>,
    lyric_line: Option<usize>,
    position_ms: u32,
    cover: Option<&Arc<RenderImage>>,
) -> AnyElement {
    // 歌词区域高度（固定高度）
    let lyric_height = px(120.0);

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
        .child(
            div().id("lyrics-area").flex_none()
                .w(px(400.0)).h(lyric_height)
                .overflow_hidden()
                .flex().flex_col().items_center().justify_center()
                .child(if let Some(ref lyrics) = lyrics {
                    if let Some(line_idx) = lyric_line {
                        // 显示当前歌词行及其上下文（前后各2行）
                        let start = line_idx.saturating_sub(2);
                        let end = (line_idx + 3).min(lyrics.lines.len());

                        div().flex_col().items_center().gap_2()
                            .children((start..end).map(|i| {
                                let line = &lyrics.lines[i];
                                let is_current = i == line_idx;

                                // 当前行且支持逐字高亮：用逐字渲染
                                if is_current {
                                    if let Some(ref highlights) = lyrics.get_word_highlights(position_ms, i) {
                                        // 逐字高亮渲染
                                        let line_el = div().flex().flex_wrap()
                                            .items_center().justify_center();
                                        let line_el = line_el.children(
                                            highlights.iter().map(|wh| {
                                                div()
                                                    .text_color(if wh.highlighted { t.accent_light } else { t.muted })
                                                    .child(wh.text.clone())
                                            })
                                        );
                                        return div().text_center()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_size(px(16.0))
                                            .child(line_el)
                                            .into_any();
                                    }
                                }

                                // 非当前行，或当前行但不支持逐字：整行渲染
                                let (color, weight, size) = if is_current {
                                    (t.accent_light, gpui::FontWeight::SEMIBOLD, px(16.0))
                                } else if i < line_idx {
                                    (t.muted_fg, gpui::FontWeight::NORMAL, px(13.0))
                                } else {
                                    (t.muted, gpui::FontWeight::NORMAL, px(13.0))
                                };

                                div().text_center()
                                    .text_color(color)
                                    .font_weight(weight)
                                    .text_size(size)
                                    .child(line.text.clone())
                                    .into_any()
                            }))
                            .into_any()
                    } else {
                        // 有歌词但未到第一句：显示前两行歌词（全暗色）
                        let preview_count = 2.min(lyrics.lines.len());
                        div().flex_col().items_center().gap_2()
                            .children((0..preview_count).map(|i| {
                                div().text_center()
                                    .text_color(t.muted)
                                    .font_weight(gpui::FontWeight::NORMAL)
                                    .text_size(px(13.0))
                                    .child(lyrics.lines[i].text.clone())
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
        .into_any()
}
