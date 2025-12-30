//! # 统一错误处理模块
//!
//! 定义 Qutility 的所有错误类型，使用 `thiserror` 派生。
//!
//! ## 依赖关系
//! - 被所有其他模块使用
//! - 无外部模块依赖

use thiserror::Error;

/// Qutility 统一错误类型
#[derive(Error, Debug)]
pub enum QutilityError {
    // ─────────────────────────────────────────────────────────────
    // I/O 错误
    // ─────────────────────────────────────────────────────────────
    #[error("Failed to read file: {path}")]
    FileReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write file: {path}")]
    FileWriteError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    // ─────────────────────────────────────────────────────────────
    // 解析错误
    // ─────────────────────────────────────────────────────────────
    #[error("Failed to parse {format} file: {path}\nReason: {reason}")]
    ParseError {
        format: String,
        path: String,
        reason: String,
    },

    #[error("Invalid structure format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    // ─────────────────────────────────────────────────────────────
    // 转换错误
    // ─────────────────────────────────────────────────────────────
    #[error("Conversion failed: {from} -> {to}\nReason: {reason}")]
    ConversionError {
        from: String,
        to: String,
        reason: String,
    },

    // ─────────────────────────────────────────────────────────────
    // 外部命令错误
    // ─────────────────────────────────────────────────────────────
    #[error("External command '{command}' not found in PATH")]
    CommandNotFound { command: String },

    #[error("External command failed: {command}\n{stderr}")]
    CommandFailed { command: String, stderr: String },

    // ─────────────────────────────────────────────────────────────
    // 参数错误
    // ─────────────────────────────────────────────────────────────
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Invalid range format: {0}")]
    InvalidRange(String),

    // ─────────────────────────────────────────────────────────────
    // CSV 错误
    // ─────────────────────────────────────────────────────────────
    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),

    // ─────────────────────────────────────────────────────────────
    // 其他
    // ─────────────────────────────────────────────────────────────
    #[error("No matching files found with pattern: {pattern}")]
    NoFilesFound { pattern: String },

    #[error("{0}")]
    Other(String),
}

/// Result 类型别名
pub type Result<T> = std::result::Result<T, QutilityError>;
