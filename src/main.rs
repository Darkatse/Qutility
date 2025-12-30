//! # Qutility - 计算凝聚态物理统一工具箱
//!
//! 将分散的计算辅助脚本用 Rust 重构，统一成单一可执行文件。
//!
//! ## 子命令
//! - `convert` - 结构格式转换 (.res, .cell, .cif, POSCAR)
//! - `analyze` - 分析功能
//!   - `dft` - DFT 计算结果分析
//!   - `xrd` - XRD 衍射图样计算
//! - `collect` - 收集完成的 DFT 计算结果
//! - `submit`  - 批量提交作业到 Slurm
//!
//! ## 依赖关系
//! ```text
//! main.rs
//!   ├── cli/        (命令行参数定义)
//!   ├── commands/   (命令执行逻辑)
//!   │     ├── parsers/   (格式解析器)
//!   │     ├── converters/(格式转换器)
//!   │     └── models/    (数据模型)
//!   ├── utils/      (工具函数)
//!   └── error.rs    (错误处理)
//! ```

mod batch;
mod cli;
mod commands;
mod error;
mod models;
mod parsers;
mod utils;
mod xrd;

use clap::Parser;
use cli::Cli;

fn main() {
    // Initialize colored output for Windows compatibility
    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).ok();

    let cli = Cli::parse();

    if let Err(e) = commands::run(cli.command) {
        utils::output::print_error(&format!("{}", e));
        std::process::exit(1);
    }
}
