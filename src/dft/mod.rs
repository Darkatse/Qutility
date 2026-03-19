//! # DFT 共享领域模块
//!
//! 提供 VASP/CASTEP 作业扫描、状态分类与重算候选筛选能力。
//!
//! ## 依赖关系
//! - 被 `commands/analyze/` 与 `commands/collect.rs` 复用
//! - 使用 `models/calculation.rs` 与 `parsers/`

mod scan;

pub use scan::{retry_candidates, scan_calculations, RetryScope};
