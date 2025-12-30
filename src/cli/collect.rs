//! # collect 子命令 CLI 定义
//!
//! 收集完成的 DFT 计算结果并转换为 .res 格式
//!
//! ## 依赖关系
//! - 被 `cli/mod.rs` 使用
//! - 参数传递给 `commands/collect.rs`

use super::analyze::DftCode;
use clap::Args;
use std::path::PathBuf;

/// collect 子命令参数
#[derive(Args, Debug)]
pub struct CollectArgs {
    /// Path to the root directory containing DFT calculation folders
    pub dft_dir: PathBuf,

    /// Specify the DFT code used
    #[arg(long, value_enum)]
    pub code: DftCode,

    /// Filename for the final concatenated .res file
    #[arg(long, default_value = "all_structures.res")]
    pub output: PathBuf,

    /// Use external 'cabal' command for conversion
    #[arg(long, default_value_t = false)]
    pub use_cabal: bool,
}
