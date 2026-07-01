// ─── 播放控制栏模块 ───
use gpui::*;
use crate::theme::ThemeConfig;

pub fn build_player_bar(
    _playing: bool,
    progress: f64,
    cur_t: f64,
    tot_t: f64,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    div().id("bar")
        .absolute().bottom(px(0.0)).left(px(0.0)).right(px(0.0))
        .flex_col().px_12().pt_4().pb_6()
        .border_t_1().border_color(t.border)
        // ── 进度条 + 时间（统一居中容器）──
        .child(
            div().flex_col().items_center().w_full()
                // 进度条
                .child(
                    div().id("prog-track").w(px(400.0)).h(px(4.0)).rounded_full().bg(t.border).relative()
                        .child(
                            div().id("prog-fill").absolute().top(px(0.0)).left(px(0.0))
                                .h(px(4.0)).rounded_full().bg(t.accent_light)
                                .w(px((progress as f32 * 400.0).min(400.0).max(0.0)))
                        )
                )
                // 时间（与进度条等宽对齐）
                .child(
                    div().flex().justify_between().w(px(400.0)).mt_2()
                        .child(div().text_xs().text_color(t.muted_fg)
                            .child(crate::audio::fmt_time(cur_t)))
                        .child(div().text_xs().text_color(t.muted_fg)
                            .child(crate::audio::fmt_time(tot_t)))
                )
        )
        // ── 控制按钮 ──
        .child(
            div().flex().items_center().justify_center().gap_6().mt_5()
                // 切换音乐库
                .child(
                    div().id("c-toggle-lib").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .on_click(cx.listener(crate::MusicPlayer::toggle_sidebar))
                        .child(svg().path("icons/list.svg").size_4().text_color(t.accent_light))
                )
                // 上一曲
                .child(
                    div().id("c-prev").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.text_color(t.fg))
                        .on_click(cx.listener(crate::MusicPlayer::prev))
                        .child(svg().path("icons/prev.svg").size_4().text_color(t.muted_fg))
                )
                // 播放/暂停（大按钮）
                .child(
                    div().id("c-play").w(px(64.0)).h(px(64.0)).rounded_full()
                        .bg(t.play_btn).flex().items_center().justify_center()
                        .cursor_pointer()
                        .shadow_lg()
                        .hover(|style| style.opacity(0.9))
                        .active(|s| s.opacity(0.8))
                        .on_click(cx.listener(crate::MusicPlayer::play_pause))
                        .child(svg().path(if _playing { "icons/pause.svg" } else { "icons/play.svg" }).size_6().text_color(t.play_btn_fg))
                )
                // 下一曲
                .child(
                    div().id("c-next").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.text_color(t.fg))
                        .on_click(cx.listener(crate::MusicPlayer::next))
                        .child(svg().path("icons/next.svg").size_4().text_color(t.muted_fg))
                )
                // 随机播放
                .child(
                    div().id("c-shuffle").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .child(svg().path("icons/shuffle.svg").size_4().text_color(t.accent_light))
                )
        )
        .into_any()
}
