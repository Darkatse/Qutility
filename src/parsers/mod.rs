//! # 解析器模块
//!
//! 提供各种结构文件和 DFT 输出格式的解析器。
//!
//! ## 依赖关系
//! - 被 `commands/` 模块使用
//! - 使用 `models/` 数据模型
//! - 子模块: res, cell, poscar, cif, outcar, castep_out

pub mod castep_out;
pub mod cell;
pub mod outcar;
pub mod poscar;
pub mod res;

use crate::error::{QutilityError, Result};
use crate::models::Crystal;
use std::path::Path;

/// 从文件路径推断格式并解析
pub fn parse_structure_file(path: &Path) -> Result<Crystal> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "res" => res::parse_res_file(path),
        "cell" => cell::parse_cell_file(path),
        _ => {
            // 可能是 POSCAR/CONTCAR (无扩展名)
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("POSCAR") || name.starts_with("CONTCAR") {
                    return poscar::parse_poscar_file(path);
                }
            }
            Err(QutilityError::UnsupportedFormat(format!(
                "Cannot determine format for: {}",
                path.display()
            )))
        }
    }
}
