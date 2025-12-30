//! # convert 子命令 CLI 定义
//!
//! 批量转换结构文件格式 (.res -> .cell/.cif/.xyz/POSCAR)
//!
//! ## 依赖关系
//! - 被 `cli/mod.rs` 使用
//! - 参数传递给 `commands/convert.rs`

use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// 支持的输出格式
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    /// CASTEP .cell format
    Cell,
    /// Crystallographic Information File
    Cif,
    /// XYZ format
    Xyz,
    /// XTL format (CrystalMaker)
    Xtl,
    /// VASP POSCAR format
    Poscar,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Cell => write!(f, "cell"),
            OutputFormat::Cif => write!(f, "cif"),
            OutputFormat::Xyz => write!(f, "xyz"),
            OutputFormat::Xtl => write!(f, "xtl"),
            OutputFormat::Poscar => write!(f, "poscar"),
        }
    }
}

/// convert 子命令参数
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Input directory containing structure files
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output directory for converted files
    #[arg(short, long)]
    pub output: PathBuf,

    /// Target output format
    #[arg(short, long, value_enum)]
    pub target: OutputFormat,

    /// Recurse into subdirectories
    #[arg(short, long, default_value_t = false)]
    pub recursive: bool,

    /// Glob pattern for input files
    #[arg(short, long, default_value = "*.res")]
    pub pattern: String,

    /// Number of parallel jobs (0 = auto)
    #[arg(short, long, default_value_t = 0)]
    pub jobs: usize,

    /// Apply Niggli reduction (requires 'cabal' in PATH)
    #[arg(long, default_value_t = false)]
    pub niggli: bool,

    /// Overwrite existing output files
    #[arg(long, default_value_t = false)]
    pub overwrite: bool,

    /// Use external 'cabal' command for conversion (fallback mode)
    #[arg(long, default_value_t = false)]
    pub use_cabal: bool,
}
