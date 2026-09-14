// ─── 播放队列面板（第三组 UI 设计的第一个面板）───
// 说明：本轮只做 UI —— 列表内容按"当前曲目之后的顺序曲目"展示，行本身不接点击播放，
// 后续接功能时把 on_click 指向 play(fi, ti) 即可。
use gpui::*;
// when 来自 FluentBuilder；overflow_y_scroll 来自 StatefulInteractiveElement
use gpui::prelude::{FluentBuilder, StatefulInteractiveElement};
use crate::theme::ThemeConfig;

/// 队列面板宽度（和设置面板一样做响应式，这里只给默认值）
pub fn build_queue_panel(
    playing: bool,
    eq_phase: u32,
    cur_title: &str,
    cur_artist: &str,
    upcoming: &[(String, String, String)],
    t: &ThemeConfig,
    panel_width: gpui::Pixels,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    // 「正在播放」里的均衡器动画条（沿用侧边栏播放中那一套相位值）
    let phase = eq_phase as f32 * 0.6;
    let bar_h = [4.0 + 8.0 * (phase + 0.0).sin().abs(),
                 4.0 + 10.0 * (phase + 1.2).sin().abs(),
                 4.0 + 8.0 * (phase + 2.4).sin().abs()];

    // 正在播放卡片
    let now_playing = div().mx_4().px_3().py_2().rounded_lg()
        .bg(Hsla { h: t.accent.h, s: t.accent.s, l: t.accent.l, a: 0.12 })
        .flex().items_center().gap_3()
        // 封面占位（真实封面后续再接，这里保持 UI 骨架）
        .child(
            div().flex_none().w(px(36.0)).h(px(36.0)).rounded_lg()
                .bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.06 })
                .flex().items_center().justify_center()
                .child(svg().path("icons/music.svg").size_4().text_color(t.accent_light))
        )
        .child(
            div().flex_1().flex_col().min_w(px(0.0))
                .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(t.fg).text_ellipsis().child(cur_title.to_string()))
                .child(div().text_xs().text_color(t.muted_fg)
                    .text_ellipsis().child(cur_artist.to_string()))
        )
        // 播放中：三条跳动竖条
        .child(
            div().flex_none().w(px(18.0)).h(px(16.0))
                .flex().items_end().justify_end().gap(px(2.0))
                .children(bar_h.iter().map(|h| {
                    div().w(px(3.0)).h(px(*h)).rounded_full().bg(t.accent_light).into_any()
                }))
                .when(!playing, |this| this.opacity(0.35))
        );

    // 「接下来」列表行
    let rows: Vec<AnyElement> = if upcoming.is_empty() {
        vec![div().mx_4().px_3().py_6().text_xs().text_color(t.muted_fg)
            .text_center().child("队列里没有更多歌曲").into_any()]
    } else {
        upcoming.iter().enumerate().map(|(i, (title, artist, dur))| {
            div().id(("q-row", i as u64)).mx_4().px_3().py_2().rounded_lg()
                .flex().items_center().gap_3()
                .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.05 }))
                .child(div().flex_none().w(px(18.0)).text_xs().text_color(t.muted_fg)
                    .child(format!("{}", i + 1)))
                .child(
                    div().flex_1().flex_col().min_w(px(0.0))
                        .child(div().text_sm().text_color(t.fg).text_ellipsis().child(title.clone()))
                        .child(div().text_xs().text_color(t.muted_fg).text_ellipsis().child(artist.clone()))
                )
                .child(div().flex_none().text_xs().text_color(t.muted_fg).child(dur.clone()))
                .into_any()
        }).collect()
    };

    div().id("queue-panel").flex_none().w(panel_width).flex_col()
        .bg(Hsla { h: t.bg.h, s: t.bg.s, l: t.bg.l, a: 0.4 })
        .border_l_1().border_color(t.border)
        // ── 头部：标题 + 清空 + 关闭 ──
        .child(
            div().flex().items_center().gap_2().px_5().pt_5().pb_2()
                .child(svg().path("icons/queue.svg").size_5().text_color(t.accent_light))
                .child(div().flex_1().text_lg().font_weight(gpui::FontWeight::BOLD)
                    .text_color(t.fg).child("播放队列"))
                // 清空队列（UI 占位，未接功能）
                .child(
                    div().id("queue-clear").px_2().py_1().rounded_md().text_xs()
                        .text_color(t.muted_fg).cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }).text_color(t.fg))
                        .child("清空")
                )
                // 关闭面板
                .child(
                    div().id("queue-close").w(px(24.0)).h(px(24.0)).rounded_md()
                        .flex().items_center().justify_center().cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.08 }))
                        .on_click(cx.listener(|this, _e, _w, cx| {
                            this.queue_open = false;
                            cx.notify();
                        }))
                        .child(svg().path("icons/close.svg").size_4().text_color(t.muted_fg))
                )
        )
        // ── 列表区（可滚动）──
        // 注意：overflow_y_scroll 定义在 StatefulInteractiveElement 上，只有带 id 的
        // div（即 Stateful<Div>）才能用，所以这里必须 .id(...)。
        .child(
            div().id("queue-list").flex_col().gap_1().flex_1().overflow_y_scroll()
                .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.muted_fg).px_5().mt_3().mb_1().child("正在播放"))
                .child(now_playing)
                .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.muted_fg).px_5().mt_4().mb_1()
                    .child(format!("接下来 · {} 首", upcoming.len())))
                .children(rows)
                .child(div().h(px(12.0)))
        )
        .into_any()
}
