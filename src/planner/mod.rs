pub mod audio;
pub mod compress;
pub mod mute;
pub mod utils;

use std::ffi::OsString;
use std::path::PathBuf;

pub use audio::plan_audio;
pub use compress::{
    CompressMode, CompressPlan, PlannedEncode, QualitySearch, VideoEncoder, VideoSource,
    compress_output_path, plan_compress, plan_compress_mode,
};
pub use mute::plan_mute;

#[derive(Debug, PartialEq)]
pub struct ExecutionPlan {
    pub input: PathBuf,
    pub final_output: PathBuf,
    pub overwrite: bool,
    pub ffmpeg_args: Vec<OsString>,
}
