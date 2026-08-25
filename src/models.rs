// ─── 数据模型 ───
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// 单首曲目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    /// 专辑封面缩略图路径（持久化到磁盘的缩略图文件路径）
    /// 运行时通过这个路径加载封面图片
    pub cover_path: Option<String>,
    /// 运行时加载的封面（不序列化）
    #[serde(skip)]
    #[allow(dead_code)]
    pub cover: Option<Arc<Vec<u8>>>,
}

/// 文件夹（包含多首曲目）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub name: String,
    /// 在「播放列表」视图里是否展开显示其中的歌曲
    pub expanded: bool,
    pub tracks: Vec<Track>,
}
