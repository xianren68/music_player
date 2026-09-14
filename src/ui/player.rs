// ─── 播放控制栏模块 ───
use gpui::*;
use std::sync::{Arc, Mutex};
use crate::theme::ThemeConfig;

/// 进度轨道固定宽度（填充条宽度、拖动点位置都按它换算，所以用固定值而不是 flex_1）
const TRACK_W: f32 = 360.0;
/// 拖动点直径
const THUMB_D: f32 = 12.0;

/// 把窗口 x 坐标换算成轨道内的 0~1 比例。
/// GPUI 的点击/移动事件只给窗口坐标、不给元素边界，所以轨道边界由 canvas
/// 在 paint 阶段写进共享 slot，这里再据此换算。
fn frac_from(slot: &Arc<Mutex<Option<Bounds<Pixels>>>>, x: Pixels) -> Option<f32> {
    let bounds = match slot.lock() {
        Ok(g) => g.clone(),
        Err(_) => return None,
    }?;
    let left = f32::from(bounds.origin.x);
    let width = f32::from(bounds.size.width);
    if width <= 0.0 {
        return None;
    }
    Some(((f32::from(x) - left) / width).clamp(0.0, 1.0))
}

pub fn build_player_bar(
    playing: bool,
    cur_t: f64,
    tot_t: f64,
    shuffle: bool,
    queue_open: bool,
    track_bounds: Arc<Mutex<Option<Bounds<Pixels>>>>,
    t: &ThemeConfig,
    cx: &mut Context<crate::MusicPlayer>,
) -> AnyElement {
    // 进度百分比（0.0 ~ 1.0）
    let pct: f32 = if tot_t > 0.0 { (cur_t / tot_t).min(1.0).max(0.0) as f32 } else { 0.0 };
    // 填充条宽度 + 拖动点位置（轨道宽度固定，可直接算出来）
    let fill_w = TRACK_W * pct;
    let thumb_left = (fill_w - THUMB_D / 2.0).clamp(0.0, TRACK_W - THUMB_D);

    // ── 进度轨道：点击跳转 + 按住拖动（seek）──
    let track = div().id("prog-track")
        .flex_none().w(px(TRACK_W)).h(px(16.0))
        .relative()
        .cursor_pointer()
        // 按下即跳转（on_mouse_down 第一个参数是按键，第二个才是回调）
        .on_mouse_down(MouseButton::Left, cx.listener({
            let slot = track_bounds.clone();
            move |this, e: &MouseDownEvent, _w, cx| {
                if let Some(frac) = frac_from(&slot, e.position.x) {
                    this.seek_fraction(frac, cx);
                }
            }
        }))
        // 按住左键拖动 = 连续 seek（pressed_button 由 GPUI 填，无需自己维护拖拽状态）
        .on_mouse_move(cx.listener({
            let slot = track_bounds.clone();
            move |this, e: &MouseMoveEvent, _w, cx| {
                if e.pressed_button != Some(MouseButton::Left) { return; }
                if let Some(frac) = frac_from(&slot, e.position.x) {
                    this.seek_fraction(frac, cx);
                }
            }
        }))
        // ① 边界捕获（放最底层，只为拿到本元素的矩形）
        .child(
            canvas(
                |bounds, _w, _cx| bounds,
                {
                    let slot = track_bounds.clone();
                    move |_bounds, out, _w, _cx| {
                        if let Ok(mut g) = slot.lock() { *g = Some(out); }
                    }
                },
            )
            .absolute().left(px(0.0)).top(px(0.0))
            .w(px(TRACK_W)).h(px(16.0))
        )
        // ② 可见轨道（垂直居中）
        .child(
            div().absolute().left(px(0.0)).top(px(5.5))
                .w(px(TRACK_W)).h(px(5.0)).rounded_full()
                .bg(t.border).overflow_hidden()
                .child(div().h(px(5.0)).rounded_full().bg(t.accent_light).w(px(fill_w)))
        )
        // ③ 拖动点
        .child(
            div().absolute().left(px(thumb_left)).top(px(2.0))
                .w(px(THUMB_D)).h(px(THUMB_D)).rounded_full()
                .bg(t.accent_light)
                .shadow_lg()
        );

    div().id("bar")
        .flex_none().flex_col().items_center().px_6().py_4()
        // ── 进度行：当前时间 | 轨道 | 总时长（原型就是横向一条）──
        .child(
            div().id("progress-row").flex().items_center().justify_center()
                .gap_3().w_full().max_w(px(460.0))
                .child(div().flex_none().w(px(40.0)).text_xs().text_center()
                    .text_color(t.muted_fg).child(crate::audio::fmt_time(cur_t)))
                .child(track)
                .child(div().flex_none().w(px(40.0)).text_xs().text_center()
                    .text_color(t.muted_fg).child(crate::audio::fmt_time(tot_t)))
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
                // 随机播放（开启时强调色高亮）
                .child(
                    div().id("c-shuffle").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.06 }))
                        .on_click(cx.listener(crate::MusicPlayer::toggle_shuffle))
                        .child(svg().path("icons/shuffle.svg").size_4()
                            .text_color(if shuffle { t.accent_light } else { t.muted_fg }))
                )
                // 播放队列（面板打开时强调色高亮）
                .child(
                    div().id("c-queue").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.06 }))
                        .on_click(cx.listener(crate::MusicPlayer::toggle_queue))
                        .child(svg().path("icons/queue.svg").size_4()
                            .text_color(if queue_open { t.accent_light } else { t.muted_fg }))
                )
                // 迷你播放器（第四组）：窗口缩成 420×120 的紧凑条
                .child(
                    div().id("c-mini").w(px(36.0)).h(px(36.0)).rounded_full()
                        .flex().items_center().justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(Hsla { h: 0.0, s: 0.0, l: 1.0, a: 0.06 }))
                        .on_click(cx.listener(crate::MusicPlayer::toggle_mini_mode))
                        .child(svg().path("icons/mini.svg").size_4().text_color(t.muted_fg))
                )
        )
        .into_any()
}
