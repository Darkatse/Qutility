//! # XRD 计算模块
//!
//! 提供 X 射线衍射图样计算功能。
//!
//! ## 子模块
//! - `scattering`: 原子散射因子数据库
//! - `calculator`: XRD 衍射峰计算
//! - `plot`: 图表生成
//! - `export`: 数据导出
//!
//! ## 依赖关系
//! - 被 `commands/analyze/xrd.rs` 使用
//! - 使用 `models/structure.rs`

pub mod calculator;
pub mod export;
pub mod plot;
pub mod scattering;

pub use calculator::{Peak, XrdCalculator, XrdPattern};
