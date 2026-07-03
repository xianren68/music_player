// ─── 播放控制栏模块 ───
use gpui::*;
use crate::theme::ThemeConfig;

pub fn build_player_bar(
    playing: bool,
    _progress: f64,
    cur_t: f64,
    tot_t: f64,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    // 进度百分比（0.0 ~ 1.0），用于控制进度条宽度
    let pct: f32 = if tot_t > 0.0 { (cur_t / tot_t).min(1.0).max(0.0) as f32 } else { 0.0 };

    div().id("bar")
        .flex_none().flex_col().items_center().px_6().py_4()
        // ── 进度条 + 时间（居中容器）──
        .child(
            div().id("progress-wrap").flex_col().items_center()
                .w_full().max_w(px(420.0))
                // 进度条：用 overflow_hidden 裁剪，填充条用正常流（不用 absolute）
                .child(
                    div().id("prog-track")
                        .w_full().h(px(5.0)).rounded_full()
                        .bg(t.border).overflow_hidden()
                        // 填充条：宽度由百分比直接控制
                        .child(
                            div().id("prog-fill")
                                .h(px(5.0)).rounded_full()
                                .bg(t.accent_light)
                                // 宽度 = 进度百分比，GPUI w() 需要 f32
                                .w(px((pct * 420.0_f32).max(0.0).min(420.0)))
                        )
                )
                // 时间文字
                .child(
                    div().id("time-row").flex().justify_between().w_full().mt_2()
                        .child(div().text_xs().text_color(t.muted_fg)
                            .child(crate::audio::fmt_time(cur_t)))
                        .child(div().text_xs().text_color(t.muted_fg)
                            .child(crate::audio::fmt_time(tot_t)))
                )
        )
        // ── 控制按钮 ──
        .child(
            div().id("controls").flex().items_center().justify_center()
                .gap_6().mt_5()
                // 切换音乐库
                .child(
                    div().id("c-toggle-lib").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .on_click(cx.listener(crate::MusicPlayer::toggle_sidebar))
                        .child(svg().path("icons/list.svg").size_4()
                            .text_color(t.accent_light))
                )
                // 上一曲
                .child(
                    div().id("c-prev").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.text_color(t.fg))
                        .on_click(cx.listener(crate::MusicPlayer::prev))
                        .child(svg().path("icons/prev.svg").size_4()
                            .text_color(t.muted_fg))
                )
                // 播放/暂停（白色圆形大按钮）
                .child(
                    div().id("c-play").w(px(64.0)).h(px(64.0)).rounded_full()
                        .bg(t.play_btn).flex().items_center().justify_center()
                        .cursor_pointer()
                        .shadow_lg()
                        .hover(|style| style.opacity(0.9))
                        .active(|s| s.opacity(0.8))
                        .on_click(cx.listener(crate::MusicPlayer::play_pause))
                        .child(svg().path(if playing {
                            "icons/pause.svg"
                        } else {
                            "icons/play.svg"
                        }).size_6().text_color(t.play_btn_fg))
                )
                // 下一曲
                .child(
                    div().id("c-next").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.text_color(t.fg))
                        .on_click(cx.listener(crate::MusicPlayer::next))
                        .child(svg().path("icons/next.svg").size_4()
                            .text_color(t.muted_fg))
                )
                // 随机播放
                .child(
                    div().id("c-shuffle").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .child(svg().path("icons/shuffle.svg").size_4()
                            .text_color(t.accent_light))
                )
        )
        .into_any()
}
