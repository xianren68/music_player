// ─── 数据模型 ───
use std::path::PathBuf;

/// 单曲信息
#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
}

/// 文件夹（包含多首曲目）
pub struct Folder {
    pub name: String,
    pub expanded: bool,
    pub tracks: Vec<Track>,
}
