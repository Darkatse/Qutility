//! # analyze 命令实现
//!
//! 分析功能统一入口，包含多个子命令：
//! - `dft`: DFT 计算结果分析
//! - `xrd`: X 射线衍射图样计算
//!
//! ## 依赖关系
//! - 使用 `cli/analyze.rs` 定义的参数
//! - 子模块: dft, xrd

pub mod dft;
pub mod xrd;

use crate::cli::analyze::{AnalyzeArgs, AnalyzeCommands};
use crate::error::Result;

/// 执行 analyze 命令
pub fn execute(args: AnalyzeArgs) -> Result<()> {
    match args.command {
        AnalyzeCommands::Dft(dft_args) => dft::execute(dft_args),
        AnalyzeCommands::Xrd(xrd_args) => xrd::execute(xrd_args),
    }
}
