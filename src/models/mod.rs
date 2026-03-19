//! # 数据模型模块
//!
//! 定义结构、DFT 结果与作业状态的共享模型。
//!
//! ## 依赖关系
//! - 被 `parsers/`、`dft/`、`commands/` 使用
//! - 子模块: structure, calculation

pub mod calculation;
pub mod structure;

pub use calculation::{CalculationScanRecord, CalculationStatus, DftCodeType, DftResult};
pub use structure::{Atom, Crystal, Lattice};
