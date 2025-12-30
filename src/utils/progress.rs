//! # 进度条工具
//!
//! 封装 `indicatif` 提供统一的进度条样式。
//!
//! ## 依赖关系
//! - 被 `commands/` 模块使用
//! - 使用 `indicatif` crate

use indicatif::{ProgressBar, ProgressStyle};

/// 创建标准进度条
pub fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// 创建 spinner（用于不确定进度的任务）
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {elapsed_precise} {msg}")
            .unwrap()
            .tick_strings(&["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// 创建简单的计数进度条
pub fn create_simple_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::with_template("{bar:40.green/white} {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb
}
