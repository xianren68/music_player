// ─── 系统托盘 / 单实例 / 文件关联（Windows 专用）───
//
// 托盘实现要点（都是 Win32 的老套路，注释里写清楚每一步在干什么）：
//   1. 在后台线程注册一个窗口类，创建一个"message-only"窗口（父窗口传 HWND_MESSAGE，
//      这种窗口不显示、只收消息），用它当托盘图标的宿主。
//   2. Shell_NotifyIconW(NIM_ADD) 把图标挂到托盘，并指定回调消息 WM_TRAYICON。
//   3. 窗口过程收到 WM_TRAYICON：左键点击 → 发"显示/隐藏窗口"；右键 → 弹菜单。
//   4. 菜单点击走 WM_COMMAND，用菜单项 ID 区分，同样翻译成事件发回主线程。
// 主线程只认 `TrayEvent` 枚举，不碰任何 Win32 细节（接收循环见 main.rs）。
use std::sync::mpsc::{Receiver, Sender};
use std::sync::OnceLock;
/// 托盘菜单 / 图标点击 → 主线程的事件
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    /// 左键点图标 / 菜单"显示窗口"：窗口最小化就还原，可见就最小化
    ToggleWindow,
    /// 菜单：播放 / 暂停
    PlayPause,
    /// 菜单：下一曲
    Next,
    /// 菜单：上一曲
    Prev,
    /// 菜单：退出
    Quit,
}

/// 托盘线程 → 主线程的全局发送端（窗口过程是 C 回调，拿不到闭包，只能用静态变量）
static TRAY_TX: OnceLock<Sender<TrayEvent>> = OnceLock::new();

#[cfg(windows)]
mod imp {
    // Win32 调用都写在 unsafe fn 里；Rust 2024 要求 unsafe fn 体内再套一层 unsafe 块
    // （unsafe_op_in_unsafe_fn）。这些函数的整个函数体本来就是 unsafe 上下文，
    // 逐行再包一层不增加任何安全性，这里统一放行，保持代码可读。
    #![allow(unsafe_op_in_unsafe_fn)]
    use super::{TrayEvent, TRAY_TX};
    use std::sync::mpsc::{channel, Receiver};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
        DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage,
        RegisterClassW, SetForegroundWindow, TrackPopupMenu, TranslateMessage, HWND_MESSAGE,
        IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
        TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WM_APP, WM_COMMAND, WM_DESTROY,
        WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP,
    };

    /// 托盘图标的 ID
    const TRAY_UID: u32 = 1;
    /// 自定义托盘回调消息：WM_APP + 1
    const WM_TRAYICON: u32 = WM_APP + 1;
    /// 菜单项 ID
    const ID_SHOW: usize = 1;
    const ID_PLAYPAUSE: usize = 2;
    const ID_NEXT: usize = 3;
    const ID_PREV: usize = 4;
    const ID_QUIT: usize = 5;

    /// 启动托盘线程，返回事件接收端
    pub fn start() -> Receiver<TrayEvent> {
        let (tx, rx) = channel::<TrayEvent>();
        let _ = TRAY_TX.set(tx);
        std::thread::Builder::new()
            .name("sonic-tray".to_string())
            .spawn(move || unsafe { run_message_loop() })
            .ok();
        rx
    }

    /// 把事件发给主线程（窗口过程里调用）
    fn emit(evt: TrayEvent) {
        if let Some(tx) = TRAY_TX.get() {
            let _ = tx.send(evt);
        }
    }

    /// 托盘线程主体：注册窗口类 → 建 message-only 窗口 → 挂图标 → 消息循环
    unsafe fn run_message_loop() {
        let hinstance: HINSTANCE = match GetModuleHandleW(PCWSTR::null()) {
            Ok(h) => HINSTANCE(h.0),
            Err(_) => {
                eprintln!("[tray] GetModuleHandleW 失败，托盘不可用");
                return;
            }
        };

        let class_name = w!("SonicTrayWindow");
        // WNDCLASSW 只有 lpfnWndProc / hInstance / lpszClassName 是必须填的
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            eprintln!("[tray] RegisterClassW 失败，托盘不可用");
            return;
        }

        // message-only 窗口：父窗口传 HWND_MESSAGE，尺寸位置随便给 0
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Sonic"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance),
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[tray] CreateWindowExW 失败：{e:?}");
                return;
            }
        };

        // 图标：先用系统默认应用图标（想换成自己的 .ico 只改 LoadIconW 的入参）
        let hicon = match LoadIconW(None, IDI_APPLICATION) {
            Ok(i) => i,
            Err(_) => {
                eprintln!("[tray] LoadIconW 失败，跳过托盘图标");
                return;
            }
        };

        // 组装 NOTIFYICONDATAW：cbSize 必须填对，否则 Shell 直接拒绝
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: hicon,
            ..Default::default()
        };
        // 悬浮提示（UTF-16，必须以 0 结尾）
        let tip: Vec<u16> = "Sonic 音乐播放器"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let n = tip.len().min(nid.szTip.len());
        nid.szTip[..n].copy_from_slice(&tip[..n]);

        if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
            eprintln!("[tray] Shell_NotifyIconW(NIM_ADD) 失败");
            return;
        }
        eprintln!("[tray] 托盘图标已挂载");

        // 消息循环：GetMessageW 返回 0（收到 WM_QUIT）才退出
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // 退出前摘掉图标，否则托盘会留一个死图标
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }

    /// 右键菜单：动态造菜单、弹完立刻销毁（托盘菜单的标准做法）
    unsafe fn show_menu(hwnd: HWND) {
        let menu = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };
        let _ = AppendMenuW(menu, MF_STRING, ID_SHOW, w!("显示 / 隐藏窗口"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_PLAYPAUSE, w!("播放 / 暂停"));
        let _ = AppendMenuW(menu, MF_STRING, ID_PREV, w!("上一曲"));
        let _ = AppendMenuW(menu, MF_STRING, ID_NEXT, w!("下一曲"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("退出"));

        // 弹菜单前先 SetForegroundWindow，否则点菜单外面菜单不消失（Win32 老坑）
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            let _ = SetForegroundWindow(hwnd);
            let _ = TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
                pt.x,
                pt.y,
                None,
                hwnd,
                None,
            );
            // 菜单关闭后补一条空消息，让前台窗口状态正确还原
            let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        }
        let _ = DestroyMenu(menu);
    }

    /// 托盘宿主窗口的窗口过程
    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            // 托盘图标回调：lparam 低位是鼠标消息
            m if m == WM_TRAYICON => {
                match (lparam.0 as u32) & 0xFFFF {
                    WM_LBUTTONUP => emit(TrayEvent::ToggleWindow),
                    WM_RBUTTONUP => show_menu(hwnd),
                    _ => {}
                }
                LRESULT(0)
            }
            // 菜单项点击：低 16 位是菜单 ID
            m if m == WM_COMMAND => {
                match (wparam.0 & 0xFFFF) as usize {
                    ID_SHOW => emit(TrayEvent::ToggleWindow),
                    ID_PLAYPAUSE => emit(TrayEvent::PlayPause),
                    ID_PREV => emit(TrayEvent::Prev),
                    ID_NEXT => emit(TrayEvent::Next),
                    ID_QUIT => emit(TrayEvent::Quit),
                    _ => {}
                }
                LRESULT(0)
            }
            m if m == WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// 启动系统托盘，返回事件接收端；失败或非 Windows 返回 None
#[cfg(windows)]
pub fn start_tray() -> Option<Receiver<TrayEvent>> {
    Some(imp::start())
}

#[cfg(not(windows))]
pub fn start_tray() -> Option<Receiver<TrayEvent>> {
    None
}

// ── 单实例 ──

/// 命名互斥体句柄必须活到进程结束，所以放静态变量里（不要 CloseHandle，否则互斥体会被释放）
#[cfg(windows)]
static INSTANCE_MUTEX: OnceLock<isize> = OnceLock::new();

/// 是否本程序的第一个实例。
/// CreateMutexW 成功但 GetLastError == ERROR_ALREADY_EXISTS，说明已有实例持着同名互斥体。
#[cfg(windows)]
pub fn is_first_instance() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;

    unsafe {
        match CreateMutexW(None, false, w!("Global\\SonicMusicPlayerSingleInstance")) {
            Ok(handle) => {
                let _ = INSTANCE_MUTEX.set(handle.0 as isize);
                GetLastError() != ERROR_ALREADY_EXISTS
            }
            // 创建失败就当第一个实例处理，免得用户完全打不开程序
            Err(_) => true,
        }
    }
}

#[cfg(not(windows))]
pub fn is_first_instance() -> bool {
    true
}

// ── 第二实例 → 第一实例的文件传递（用临时文件，简单且不引入额外 IPC）──

/// 待播放文件的落地路径（%TEMP%/sonic_pending_open.txt）
fn pending_file_path() -> std::path::PathBuf {
    std::env::temp_dir().join("sonic_pending_open.txt")
}

/// 第二个实例把要打开的文件路径写到这里，然后自己退出
pub fn write_pending_file(path: &std::path::Path) {
    let _ = std::fs::write(pending_file_path(), path.to_string_lossy().as_bytes());
}

/// 第一个实例轮询：有请求就取走（读完立刻删除，避免重复播放）
pub fn take_pending_file() -> Option<std::path::PathBuf> {
    let p = pending_file_path();
    if !p.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&p).ok();
    let _ = std::fs::remove_file(&p);
    content
        .map(|s| std::path::PathBuf::from(s.trim()))
        .filter(|pb| !pb.as_os_str().is_empty())
}

// ── 文件关联（只写 HKCU，且只加"打开方式"候选，不劫持系统默认程序）──

/// 注册文件关联：在 HKCU\Software\Classes 下建 ProgID，并把 ProgID 塞进常见音频
/// 扩展名的 OpenWithProgids —— 效果是右键"打开方式"里出现本程序，默认程序不变。
#[cfg(windows)]
pub fn register_file_association() -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
        REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let exe = std::env::current_exe().map_err(|e| format!("取自身路径失败：{e}"))?;
    let exe_str = exe.to_string_lossy().to_string();

    // 写一个 REG_SZ 值（字符串要带结尾的 0，长度按字节算）
    fn write_sz(hkey: HKEY, name: Option<&str>, value: &str) -> Result<(), String> {
        let name_w: Vec<u16> = name
            .map(|n| n.encode_utf16().chain(std::iter::once(0)).collect())
            .unwrap_or_default();
        let val_w: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
        let name_pc = if name.is_some() {
            PCWSTR(name_w.as_ptr())
        } else {
            PCWSTR::null()
        };
        // 注意：字符串写进注册表要带结尾的 0，长度按字节算
        let bytes = unsafe {
            std::slice::from_raw_parts(val_w.as_ptr() as *const u8, val_w.len() * 2)
        };
        let rc = unsafe { RegSetValueExW(hkey, name_pc, None, REG_SZ, Some(bytes)) };
        if rc == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("RegSetValueExW 失败，错误码 {}", rc.0))
        }
    }

    let prog_id = "SonicMusicPlayer";
    // (子键路径, 值名(None=默认值), 值内容)
    let mut entries: Vec<(String, Option<&str>, String)> = vec![
        (
            format!("Software\\Classes\\{prog_id}"),
            None,
            "Sonic 音乐播放器".to_string(),
        ),
        (
            format!("Software\\Classes\\{prog_id}\\shell\\open\\command"),
            None,
            format!("\"{exe_str}\" \"%1\""),
        ),
    ];
    // OpenWithProgids 下只需要"值名存在"，内容留空
    for ext in ["mp3", "flac", "wav", "m4a", "ogg", "aac", "wma", "opus"] {
        entries.push((
            format!("Software\\Classes\\.{ext}\\OpenWithProgids"),
            Some(prog_id),
            String::new(),
        ));
    }

    for (subkey, value_name, value) in entries {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        // HKEY 没实现 Default，只能拿空指针起手
        let mut hkey = HKEY(std::ptr::null_mut());
        unsafe {
            let rc = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            );
            if rc != ERROR_SUCCESS {
                return Err(format!("创建注册表项 {subkey} 失败，错误码 {}", rc.0));
            }
            let res = write_sz(hkey, value_name, &value);
            let _ = RegCloseKey(hkey);
            res?;
        }
    }
    eprintln!("[assoc] 文件关联已写入 HKCU");
    Ok(())
}
#[cfg(not(windows))]
pub fn register_file_association() -> Result<(), String> {
    Err("仅 Windows 支持".to_string())
}
