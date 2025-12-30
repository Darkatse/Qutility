//! # 工具函数模块
//!
//! 提供美化输出、进度条、Slurm 脚本生成等工具。
//!
//! ## 依赖关系
//! - 被 `commands/` 模块使用
//! - 子模块: output, progress, slurm

pub mod output;
pub mod progress;
pub mod slurm;
