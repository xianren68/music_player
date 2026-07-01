// ─── Windows 原生窗口透明度 ───
use std::ffi::c_void;

type HWND = *mut c_void;

#[link(name = "user32")]
unsafe extern "system" {
    fn GetWindowLongW(hWnd: HWND, nIndex: i32) -> i32;
    fn SetWindowLongW(hWnd: HWND, nIndex: i32, dwNewLong: i32) -> i32;
    fn SetLayeredWindowAttributes(hWnd: HWND, crKey: u32, bAlpha: u8, dwFlags: u32) -> i32;
    fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, X: i32, Y: i32, cx: i32, cy: i32, uFlags: u32) -> i32;
}

const GWL_EXSTYLE: i32 = -20;
const WS_EX_LAYERED: i32 = 0x00080000;
const WS_EX_NOREDIRECTIONBITMAP: i32 = 0x00200000;
const LWA_ALPHA: u32 = 0x00000002;
const SWP_FRAMECHANGED: u32 = 0x0020;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOZORDER: u32 = 0x0004;

/// 设置窗口 Alpha 透明度（0.0=全透明, 1.0=不透明）
pub fn apply_alpha(hwnd: isize, alpha: f32) {
    let hwnd = hwnd as HWND;
    let b_alpha = (alpha.clamp(0.1, 1.0) * 255.0) as u8;

    unsafe {
        let mut ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        // 移除 WS_EX_NOREDIRECTIONBITMAP（与 WS_EX_LAYERED 冲突）
        ex_style &= !WS_EX_NOREDIRECTIONBITMAP;
        // 添加 WS_EX_LAYERED
        if ex_style & WS_EX_LAYERED == 0 {
            ex_style |= WS_EX_LAYERED;
        }
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0,
                     SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
        SetLayeredWindowAttributes(hwnd, 0, b_alpha, LWA_ALPHA);
    }
}

/// 恢复不透明
pub fn disable_alpha(hwnd: isize) {
    let hwnd = hwnd as HWND;
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style & !WS_EX_LAYERED);
        SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0,
                     SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
    }
}
