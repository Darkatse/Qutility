//! # DFT 计算领域模型
//!
//! 定义 DFT 结果、作业状态与扫描记录的数据结构。
//!
//! ## 依赖关系
//! - 被 `parsers/` 写入，被 `dft/` 与 `commands/` 读取
//! - 不依赖 CLI，仅承载共享领域语义

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// DFT 计算代码类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DftCodeType {
    Vasp,
    Castep,
}

impl std::fmt::Display for DftCodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DftCodeType::Vasp => write!(f, "VASP"),
            DftCodeType::Castep => write!(f, "CASTEP"),
        }
    }
}

/// DFT 作业状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalculationStatus {
    Completed,
    Failed,
    Incomplete,
    MissingOutput,
    ParseError,
}

impl std::fmt::Display for CalculationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalculationStatus::Completed => write!(f, "completed"),
            CalculationStatus::Failed => write!(f, "failed"),
            CalculationStatus::Incomplete => write!(f, "incomplete"),
            CalculationStatus::MissingOutput => write!(f, "missing-output"),
            CalculationStatus::ParseError => write!(f, "parse-error"),
        }
    }
}

/// 已解析的 DFT 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DftResult {
    /// 结构名称
    pub structure_name: String,

    /// 使用的 DFT 代码
    pub code: DftCodeType,

    /// 焓 (eV) - 恒压计算的相关量
    pub enthalpy_ev: Option<f64>,

    /// 能量 (eV)
    pub energy_ev: Option<f64>,

    /// 压力 (kBar)
    pub pressure_kbar: Option<f64>,

    /// 体积 (Å³)
    pub volume: Option<f64>,

    /// 原子数
    pub num_atoms: Option<usize>,
}

/// 单个作业目录的扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationScanRecord {
    /// 结构名称
    pub structure_name: String,
    /// 作业目录
    pub job_dir: PathBuf,
    /// 使用的 DFT 代码
    pub code: DftCodeType,
    /// 扫描得到的状态
    pub status: CalculationStatus,
    /// 状态说明或失败原因
    pub reason: Option<String>,
    /// 结构文件路径（CONTCAR / POSCAR / .cell）
    pub structure_file: Option<PathBuf>,
    /// 已解析结果，仅在可解析时存在
    pub parsed: Option<DftResult>,
}

impl DftResult {
    pub fn new(structure_name: impl Into<String>, code: DftCodeType) -> Self {
        DftResult {
            structure_name: structure_name.into(),
            code,
            enthalpy_ev: None,
            energy_ev: None,
            pressure_kbar: None,
            volume: None,
            num_atoms: None,
        }
    }

    /// 计算每原子焓
    pub fn enthalpy_per_atom(&self) -> Option<f64> {
        match (self.enthalpy_ev, self.num_atoms) {
            (Some(h), Some(n)) if n > 0 => Some(h / n as f64),
            _ => None,
        }
    }

    /// 计算每原子能量
    pub fn energy_per_atom(&self) -> Option<f64> {
        match (self.energy_ev, self.num_atoms) {
            (Some(e), Some(n)) if n > 0 => Some(e / n as f64),
            _ => None,
        }
    }
}

impl CalculationScanRecord {
    pub fn new(
        structure_name: impl Into<String>,
        job_dir: PathBuf,
        code: DftCodeType,
        status: CalculationStatus,
    ) -> Self {
        Self {
            structure_name: structure_name.into(),
            job_dir,
            code,
            status,
            reason: None,
            structure_file: None,
            parsed: None,
        }
    }
}
