//! # 数据模型模块
//!
//! 定义统一的晶体结构和计算结果数据模型。
//!
//! ## 依赖关系
//! - 被 `parsers/` 和 `commands/` 使用
//! - 子模块: structure, calculation

pub mod calculation;
pub mod structure;

pub use calculation::{DftCodeType, DftResult};
pub use structure::{Atom, Crystal, Lattice};
