//! # DFT 计算结果数据模型
//!
//! 存储 VASP/CASTEP 计算结果的提取信息。
//!
//! ## 依赖关系
//! - 被 `parsers/outcar.rs`, `parsers/castep_out.rs` 使用
//! - 被 `commands/analyze.rs`, `commands/collect.rs` 使用

use serde::{Deserialize, Serialize};

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

/// DFT 计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DftResult {
    /// 结构名称
    pub structure_name: String,

    /// 使用的 DFT 代码
    pub code: DftCodeType,

    /// 计算是否完成
    pub is_finished: bool,

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

    /// 结构文件路径（CONTCAR 或 .cell）
    pub structure_file: Option<String>,
}

impl DftResult {
    pub fn new(structure_name: impl Into<String>, code: DftCodeType) -> Self {
        DftResult {
            structure_name: structure_name.into(),
            code,
            is_finished: false,
            enthalpy_ev: None,
            energy_ev: None,
            pressure_kbar: None,
            volume: None,
            num_atoms: None,
            structure_file: None,
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
