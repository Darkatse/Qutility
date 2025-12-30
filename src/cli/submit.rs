//! # submit 子命令 CLI 定义
//!
//! 批量提交 CASTEP/VASP 作业到 Slurm
//!
//! ## 依赖关系
//! - 被 `cli/mod.rs` 使用
//! - 参数传递给 `commands/submit.rs`

use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// DFT 引擎选择
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DftEngine {
    /// Auto-detect based on available files
    Auto,
    /// CASTEP
    Castep,
    /// VASP
    Vasp,
}

/// submit 子命令参数
#[derive(Args, Debug)]
pub struct SubmitArgs {
    /// Path to the CSV file containing structure list
    #[arg(long)]
    pub csv: PathBuf,

    /// Path to directory containing structure files (.cell / POSCAR)
    #[arg(long)]
    pub struct_dir: PathBuf,

    /// Range of structures to submit (e.g., '1-20,25,30-32')
    #[arg(long)]
    pub range: String,

    /// Root directory for job folders
    #[arg(long, default_value = "jobs")]
    pub jobs_root: PathBuf,

    /// DFT engine to use
    #[arg(long, value_enum, default_value = "castep")]
    pub dft: DftEngine,

    // ─────────────────────────────────────────────────────────────
    // CASTEP options
    // ─────────────────────────────────────────────────────────────
    /// CASTEP .param template file path
    #[arg(long)]
    pub param_template: Option<PathBuf>,

    /// CASTEP executable name
    #[arg(long, default_value = "castep.mpi")]
    pub castep_exec: String,

    /// Number of MPI processes for CASTEP
    #[arg(long, default_value_t = 32)]
    pub castep_np: u32,

    /// Module list for CASTEP (comma-separated)
    #[arg(long, default_value = "airss/arm-v2/0.2,castep/arm-v2/25.12")]
    pub castep_modules: String,

    /// External pressure in GPa (for CASTEP %BLOCK EXTERNAL_PRESSURE)
    #[arg(long)]
    pub external_pressure: Option<f64>,

    // ─────────────────────────────────────────────────────────────
    // VASP options
    // ─────────────────────────────────────────────────────────────
    /// VASP INCAR template file
    #[arg(long)]
    pub incar_template: Option<PathBuf>,

    /// VASP KPOINTS template file
    #[arg(long)]
    pub kpoints_template: Option<PathBuf>,

    /// VASP POTCAR library directory
    #[arg(long)]
    pub potcar_dir: Option<PathBuf>,

    /// VASP executable name
    #[arg(long, default_value = "vasp_std")]
    pub vasp_exec: String,

    /// Number of MPI processes for VASP
    #[arg(long, default_value_t = 32)]
    pub vasp_np: u32,

    /// Module list for VASP (comma-separated)
    #[arg(long, default_value = "")]
    pub vasp_modules: String,

    // ─────────────────────────────────────────────────────────────
    // Slurm options
    // ─────────────────────────────────────────────────────────────
    /// Slurm partition
    #[arg(long, default_value = "arm")]
    pub partition: String,

    /// Slurm constraint
    #[arg(long, default_value = "neoverse_v2")]
    pub constraint: String,

    /// Number of nodes
    #[arg(long, default_value_t = 1)]
    pub nodes: u32,

    /// Number of tasks
    #[arg(long, default_value_t = 32)]
    pub ntasks: u32,

    /// CPUs per task
    #[arg(long, default_value_t = 1)]
    pub cpus_per_task: u32,

    /// Memory per CPU
    #[arg(long, default_value = "3G")]
    pub mem_per_cpu: String,

    /// Time limit (e.g., '24:00:00')
    #[arg(long, default_value = "24:00:00")]
    pub time: String,

    // ─────────────────────────────────────────────────────────────
    // Execution control
    // ─────────────────────────────────────────────────────────────
    /// Only generate job files, do not submit
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Submit jobs to Slurm after generation
    #[arg(long, default_value_t = false)]
    pub submit: bool,
}
