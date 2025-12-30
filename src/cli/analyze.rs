//! # analyze 子命令 CLI 定义
//!
//! 分析功能统一入口，包含多个子命令：
//! - `dft`: DFT 计算结果分析
//! - `xrd`: X 射线衍射图样计算
//!
//! ## 依赖关系
//! - 被 `cli/mod.rs` 使用
//! - 参数传递给 `commands/analyze/` 相应模块

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────
// Analyze 主命令
// ─────────────────────────────────────────────────────────────

/// analyze 主命令参数
#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    #[command(subcommand)]
    pub command: AnalyzeCommands,
}

/// analyze 子命令
#[derive(Subcommand, Debug)]
pub enum AnalyzeCommands {
    /// Analyze DFT calculation results (VASP/CASTEP)
    Dft(DftArgs),

    /// Calculate X-ray diffraction pattern from structure
    Xrd(XrdArgs),
}

// ─────────────────────────────────────────────────────────────
// DFT 分析子命令
// ─────────────────────────────────────────────────────────────

/// DFT 计算代码类型
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DftCode {
    /// VASP
    Vasp,
    /// CASTEP
    Castep,
}

impl std::fmt::Display for DftCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DftCode::Vasp => write!(f, "vasp"),
            DftCode::Castep => write!(f, "castep"),
        }
    }
}

/// DFT 分析子命令参数
#[derive(Args, Debug)]
pub struct DftArgs {
    /// Path to the input CSV file from EDDP/Repose search
    #[arg(long)]
    pub csv_file: Option<PathBuf>,

    /// Path to the root directory containing DFT calculation folders
    #[arg(long)]
    pub job_dir: PathBuf,

    /// Specify the DFT code used
    #[arg(long, value_enum)]
    pub code: DftCode,

    /// Range of top structures to plot (e.g., '1-10')
    #[arg(long)]
    pub plot_range: Option<String>,

    /// Number of top structures to print from final DFT ranking
    #[arg(long, default_value_t = 10)]
    pub top_n: usize,

    /// Filename for the final DFT-ranked CSV output
    #[arg(long, default_value = "dft_ranked_results.csv")]
    pub output_csv: PathBuf,

    /// Filename for the comparison plot (PNG format)
    #[arg(long, default_value = "eddp_vs_dft_comparison.png")]
    pub output_plot: PathBuf,

    /// Skip plot generation
    #[arg(long, default_value_t = false)]
    pub no_plot: bool,
}

// ─────────────────────────────────────────────────────────────
// XRD 分析子命令
// ─────────────────────────────────────────────────────────────

/// 预定义辐射源波长 (Å)
pub fn get_predefined_wavelength(name: &str) -> Option<f64> {
    match name.to_lowercase().as_str() {
        "cu-ka" | "cuka" => Some(1.5418),
        "cu-ka1" | "cuka1" => Some(1.5406),
        "cu-ka2" | "cuka2" => Some(1.5444),
        "cu-kb1" | "cukb1" => Some(1.3922),
        "mo-ka" | "moka" => Some(0.7107),
        "mo-ka1" | "moka1" => Some(0.7093),
        "co-ka" | "coka" => Some(1.7903),
        "fe-ka" | "feka" => Some(1.9373),
        "cr-ka" | "crka" => Some(2.2910),
        "ag-ka" | "agka" => Some(0.5609),
        _ => None,
    }
}

/// 解析波长输入（辐射源名称或数值）
pub fn parse_wavelength(input: &str) -> Result<f64, String> {
    // 先尝试解析为预定义辐射源
    if let Some(wl) = get_predefined_wavelength(input) {
        return Ok(wl);
    }
    // 再尝试解析为数值
    input.parse::<f64>().map_err(|_| {
        format!(
            "Invalid wavelength '{}'. Use a number (e.g., 0.424589) or a name: cu-ka, mo-ka, co-ka, fe-ka, cr-ka, ag-ka",
            input
        )
    })
}

/// 峰展宽类型
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum BroadeningType {
    /// No broadening (stick pattern)
    #[default]
    None,
    /// Gaussian broadening
    Gaussian,
    /// Lorentzian broadening
    Lorentzian,
    /// Pseudo-Voigt (50% Gaussian + 50% Lorentzian)
    PseudoVoigt,
}

impl std::fmt::Display for BroadeningType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BroadeningType::None => write!(f, "none"),
            BroadeningType::Gaussian => write!(f, "gaussian"),
            BroadeningType::Lorentzian => write!(f, "lorentzian"),
            BroadeningType::PseudoVoigt => write!(f, "pseudo-voigt"),
        }
    }
}

/// XRD 图像输出格式
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum XrdOutputFormat {
    /// PNG image (publication quality)
    Png,
    /// SVG vector image
    Svg,
    /// CSV data file (2θ, intensity, hkl)
    Csv,
    /// XY data file (standard XRD format)
    Xy,
}

/// XRD 分析子命令参数
#[derive(Args, Debug)]
pub struct XrdArgs {
    /// Input: structure file or directory containing structure files
    pub input: PathBuf,

    /// Output: file path (single mode) or directory (batch mode)
    #[arg(short, long, default_value = "xrd_pattern.png")]
    pub output: PathBuf,

    /// Output format (auto-detected from extension if not specified)
    #[arg(short, long, value_enum)]
    pub format: Option<XrdOutputFormat>,

    /// X-ray wavelength: radiation source name (cu-ka, mo-ka, etc.) or value in Å (e.g., 0.424589)
    #[arg(short, long, default_value = "cu-ka")]
    pub wavelength: String,

    /// 2θ range in degrees (e.g., "5-90")
    #[arg(short, long, default_value = "5-90")]
    pub range: String,

    /// Minimum intensity threshold (0-100, relative to max peak)
    #[arg(long, default_value_t = 0.1)]
    pub threshold: f64,

    /// Peak broadening type
    #[arg(long, value_enum, default_value = "none")]
    pub broadening: BroadeningType,

    /// Full Width at Half Maximum (FWHM) for peak broadening, in degrees 2θ
    #[arg(long, default_value_t = 0.1)]
    pub fwhm: f64,

    /// Step size for broadened pattern output (degrees 2θ)
    #[arg(long, default_value_t = 0.02)]
    pub step: f64,

    /// Label peaks with Miller indices (hkl)
    #[arg(long, default_value_t = false)]
    pub label_peaks: bool,

    /// Number of top peaks to label (if --label-peaks is set)
    #[arg(long, default_value_t = 10)]
    pub label_count: usize,

    /// Figure width in pixels (for PNG) or points (for SVG)
    #[arg(long, default_value_t = 1200)]
    pub width: u32,

    /// Figure height in pixels (for PNG) or points (for SVG)
    #[arg(long, default_value_t = 800)]
    pub height: u32,

    /// Title for the plot (default: structure name)
    #[arg(long)]
    pub title: Option<String>,

    // ─────────────────────────────────────────────────────────────
    // 批量处理参数
    // ─────────────────────────────────────────────────────────────
    /// Glob pattern for input files (batch mode, e.g., "*.res,*.cell,POSCAR*")
    #[arg(long, default_value = "*.res,*.cell,POSCAR*")]
    pub pattern: String,

    /// Number of parallel jobs (0 = auto, batch mode only)
    #[arg(short, long, default_value_t = 0)]
    pub jobs: usize,

    /// Recurse into subdirectories (batch mode)
    #[arg(long, default_value_t = false)]
    pub recursive: bool,

    /// Overwrite existing output files
    #[arg(long, default_value_t = false)]
    pub overwrite: bool,
}
