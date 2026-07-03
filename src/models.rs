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
    /// 专辑封面（不序列化，运行时提取）
    #[serde(skip)]
    #[allow(dead_code)]
    pub cover: Option<Arc<Vec<u8>>>,
}

/// 文件夹（包含多首曲目）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub name: String,
    pub expanded: bool,
    pub tracks: Vec<Track>,
}
