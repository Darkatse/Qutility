//! # 命令执行模块
//!
//! 实现各子命令的业务逻辑。
//!
//! ## 依赖关系
//! - 被 `main.rs` 调用
//! - 使用 `cli/`, `parsers/`, `models/`, `utils/`
//! - 子模块: convert, analyze, collect, submit

pub mod analyze;
pub mod collect;
pub mod convert;
pub mod submit;

use crate::cli::Commands;
use crate::error::Result;

/// 执行命令
pub fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Convert(args) => convert::execute(args),
        Commands::Analyze(args) => analyze::execute(args),
        Commands::Collect(args) => collect::execute(args),
        Commands::Submit(args) => submit::execute(args),
    }
}
