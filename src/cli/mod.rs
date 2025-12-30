//! # CLI 模块
//!
//! 使用 `clap` 定义命令行参数和子命令。
//!
//! ## 命令结构
//! - `convert`: 结构格式转换
//! - `analyze`: 分析功能（嵌套子命令）
//!   - `dft`: DFT 计算结果分析
//!   - `xrd`: XRD 衍射图样计算
//! - `collect`: 收集 DFT 结果
//! - `submit`: 批量作业提交
//!
//! ## 依赖关系
//! - 被 `main.rs` 使用
//! - 子模块: convert, analyze, collect, submit

pub mod analyze;
pub mod collect;
pub mod convert;
pub mod submit;

use clap::{Parser, Subcommand};

/// Qutility - 计算凝聚态物理统一工具箱
#[derive(Parser)]
#[command(name = "qutility")]
#[command(author = "Changjiang Wu")]
#[command(version)]
#[command(about = "A unified computational condensed matter physics toolkit", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// 可用的子命令
#[derive(Subcommand)]
pub enum Commands {
    /// Convert structure files between formats (.res, .cell, .cif, POSCAR)
    Convert(convert::ConvertArgs),

    /// Analyze DFT calculation results
    Analyze(analyze::AnalyzeArgs),

    /// Collect completed DFT results and convert to .res format
    Collect(collect::CollectArgs),

    /// Submit batch jobs to Slurm scheduler
    Submit(submit::SubmitArgs),
}
