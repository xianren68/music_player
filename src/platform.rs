// ─── Windows 平台原生透明度支持 ───
// 使用 SetWindowCompositionAttribute（与 GPUI 内部相同的 API，需动态加载）

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;

    type HWND = *mut c_void;
    type BOOL = i32;

    // AccentPolicy 状态
    const ACCENT_DISABLED: i32 = 0;
    const ACCENT_ENABLE_TRANSPARENTGRADIENT: i32 = 2;
    // WCA_ACCENT_POLICY = 19
    const WCA_ACCENT_POLICY: i32 = 19;

    /// AccentPolicy 结构体（控制窗口透明效果）
    #[repr(C)]
    struct AccentPolicy {
        accent_state: i32,
        accent_flags: i32,
        gradient_color: i32,
        animation_id: i32,
    }

    /// WINDOWCOMPOSITIONATTRIBDATA
    #[repr(C)]
    struct WindowCompositionAttribData {
        attribute: i32,
        data: *mut c_void,
        size_of_data: u32,
    }

    unsafe extern "system" {
        fn GetProcAddress(hmodule: *mut c_void, name: *const u8) -> *mut c_void;
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    }

    // 函数指针类型
    type SetWindowCompositionAttributeFn =
        unsafe extern "system" fn(HWND, *mut WindowCompositionAttribData) -> BOOL;

    /// 动态加载 SetWindowCompositionAttribute
    fn load_api() -> Option<SetWindowCompositionAttributeFn> {
        unsafe {
            // user32.dll 已经加载到进程中（GPUI 在使用它）
            let module_name: Vec<u16> = "user32.dll\0".encode_utf16().collect();
            let hmod = GetModuleHandleW(module_name.as_ptr());
            if hmod.is_null() {
                return None;
            }
            let name = b"SetWindowCompositionAttribute\0";
            let ptr = GetProcAddress(hmod, name.as_ptr());
            if ptr.is_null() {
                return None;
            }
            Some(std::mem::transmute(ptr))
        }
    }

    /// 设置窗口透明度（0=全透明, 100=不透明）
    pub fn set_window_opacity(hwnd: isize, alpha_percent: u8) -> bool {
        let Some(set_attr) = load_api() else {
            return false;
        };

        let hwnd = hwnd as HWND;
        // gradient_color 的高 8 位控制透明度
        let alpha = alpha_percent as i32;
        let gradient_color = alpha << 24;

        let accent = AccentPolicy {
            accent_state: ACCENT_ENABLE_TRANSPARENTGRADIENT,
            accent_flags: 0,
            gradient_color,
            animation_id: 0,
        };

        let mut data = WindowCompositionAttribData {
            attribute: WCA_ACCENT_POLICY,
            data: &accent as *const AccentPolicy as *mut c_void,
            size_of_data: std::mem::size_of::<AccentPolicy>() as u32,
        };

        let ret = unsafe { set_attr(hwnd, &mut data) };
        ret != 0
    }

    /// 恢复不透明
    pub fn set_window_opaque(hwnd: isize) -> bool {
        let Some(set_attr) = load_api() else {
            return false;
        };

        let hwnd = hwnd as HWND;

        let accent = AccentPolicy {
            accent_state: ACCENT_DISABLED,
            accent_flags: 0,
            gradient_color: 0,
            animation_id: 0,
        };

        let mut data = WindowCompositionAttribData {
            attribute: WCA_ACCENT_POLICY,
            data: &accent as *const AccentPolicy as *mut c_void,
            size_of_data: std::mem::size_of::<AccentPolicy>() as u32,
        };

        let ret = unsafe { set_attr(hwnd, &mut data) };
        ret != 0
    }
}

/// 设置窗口透明度（0.0 ~ 1.0），仅 Windows 平台有效
pub fn set_opacity(hwnd: isize, opacity: f32) {
    #[cfg(target_os = "windows")]
    {
        let alpha = (opacity.clamp(0.0, 1.0) * 100.0) as u8;
        win::set_window_opacity(hwnd, alpha);
    }
    let _ = (hwnd, opacity);
}

/// 恢复窗口不透明
pub fn set_opaque(hwnd: isize) {
    #[cfg(target_os = "windows")]
    {
        win::set_window_opaque(hwnd);
    }
    let _ = hwnd;
}
