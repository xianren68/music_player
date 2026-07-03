// ─── 设置持久化模块 ───
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// 导入 Folder 类型用于缓存
use crate::models::Folder;

/// 可持久化的应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// 窗口透明度是否启用
    pub opacity_enabled: bool,
    /// 窗口不透明度 (0.0 ~ 1.0)
    pub opacity: f32,
    /// 背景图透明度 (0.0 ~ 1.0)
    pub bg_opacity: f32,
    /// 背景图磨砂效果
    pub bg_blur: bool,
    /// 背景图路径
    pub bg_image_path: Option<String>,
    /// 主题模式: "dark" / "light"
    pub theme_mode: String,
    /// 侧边栏是否打开
    pub sidebar_open: bool,
    /// 设置面板是否打开
    pub settings_open: bool,
    /// 音乐文件夹路径列表（支持多个）
    pub music_folders: Vec<String>,
    /// 缓存的歌曲列表（避免每次启动都重新扫描）
    #[serde(default)]
    pub cached_folders: Vec<Folder>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            opacity_enabled: false,
            opacity: 1.0,
            bg_opacity: 1.0,
            bg_blur: false,
            bg_image_path: None,
            theme_mode: "dark".into(),
            sidebar_open: true,
            settings_open: false,
            music_folders: Vec::new(),
            cached_folders: Vec::new(),
        }
    }
}

impl AppSettings {
    /// 获取设置文件路径（可执行文件同级目录）
    fn settings_path() -> Option<PathBuf> {
        std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|d| d.join("settings.json")))
    }

    /// 从文件加载设置
    pub fn load() -> Self {
        let Some(path) = Self::settings_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 保存设置到文件
    pub fn save(&self) {
        let Some(path) = Self::settings_path() else { return; };
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}
