//! # analyze 命令实现
//!
//! 分析功能统一入口，协调 DFT 状态扫描、DFT 后处理与 XRD 计算。
//!
//! ## 依赖关系
//! - 使用 `cli/analyze.rs` 定义的参数
//! - 子模块: dft_status, dft_postprocessing, xrd

pub mod dft_postprocessing;
pub mod dft_status;
pub mod xrd;

use crate::cli::analyze::{AnalyzeArgs, AnalyzeCommands};
use crate::error::Result;

/// 执行 analyze 命令
pub fn execute(args: AnalyzeArgs) -> Result<()> {
    match args.command {
        AnalyzeCommands::DftStatus(status_args) => dft_status::execute(status_args),
        AnalyzeCommands::DftPostprocessing(post_args) => dft_postprocessing::execute(post_args),
        AnalyzeCommands::Xrd(xrd_args) => xrd::execute(xrd_args),
    }
}
