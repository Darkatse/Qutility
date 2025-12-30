//! # 批量处理模块
//!
//! 提供统一的文件批量处理能力。
//!
//! ## 功能
//! - 自动检测输入类型（文件/目录）
//! - 收集匹配文件列表
//! - 并行处理
//! - 进度反馈与统计
//!
//! ## 依赖关系
//! - 被各命令模块使用
//! - 使用 `rayon` 进行并行处理
//! - 使用 `indicatif` 显示进度

pub mod collector;
pub mod runner;

pub use collector::FileCollector;
pub use runner::{BatchResult, BatchRunner, ProcessResult};
