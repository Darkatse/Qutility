//! # 美化输出工具
//!
//! 提供统一的终端输出样式。
//!
//! ## 依赖关系
//! - 被所有 `commands/` 模块使用
//! - 使用 `colored` crate

use colored::Colorize;

/// 打印成功消息
pub fn print_success(msg: &str) {
    println!("{} {}", "[OK]".green().bold(), msg);
}

/// 打印错误消息
pub fn print_error(msg: &str) {
    eprintln!("{} {}", "[ERR]".red().bold(), msg);
}

/// 打印警告消息
pub fn print_warning(msg: &str) {
    println!("{} {}", "[WARN]".yellow().bold(), msg);
}

/// 打印信息消息
pub fn print_info(msg: &str) {
    println!("{} {}", "[*]".blue().bold(), msg);
}

/// 打印跳过消息
pub fn print_skip(msg: &str) {
    println!("{} {}", "[SKIP]".dimmed(), msg);
}

/// 打印完成消息
pub fn print_done(msg: &str) {
    println!("{} {}", "[DONE]".green().bold(), msg);
}

/// 打印转换成功消息
pub fn print_conversion(from: &str, to: &str) {
    println!(
        "{} {} {} {}",
        "[OK]".green().bold(),
        from.dimmed(),
        "->".cyan(),
        to
    );
}

/// 打印标题栏
pub fn print_header(title: &str) {
    let line = "─".repeat(60);
    println!("\n{}", line.dimmed());
    println!("  {}", title.bold());
    println!("{}\n", line.dimmed());
}

/// 打印分隔线
pub fn print_separator() {
    println!("{}", "─".repeat(60).dimmed());
}
