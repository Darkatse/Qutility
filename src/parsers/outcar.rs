//! # VASP OUTCAR 解析器
//!
//! 解析 VASP 计算输出文件 OUTCAR，提取焓、能量等信息。
//!
//! ## 依赖关系
//! - 被 `commands/analyze.rs`, `commands/collect.rs` 使用
//! - 使用 `models/calculation.rs`

use crate::error::{QutilityError, Result};
use crate::models::{DftCodeType, DftResult};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// 解析 VASP OUTCAR 文件
pub fn parse_outcar(path: &Path, structure_name: &str) -> Result<DftResult> {
    let file = File::open(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let reader = BufReader::new(file);
    let mut result = DftResult::new(structure_name, DftCodeType::Vasp);

    let mut final_enthalpy: Option<f64> = None;
    let mut final_energy: Option<f64> = None;
    let mut volume: Option<f64> = None;
    let mut num_atoms: Option<usize> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        // 检查是否完成
        if line.contains("General timing and accounting informations for this job") {
            result.is_finished = true;
        }

        // 提取焓 (恒压计算的相关量)
        // "enthalpy is  TOTEN    =      -123.456789 eV"
        if line.contains("enthalpy is  TOTEN") {
            if let Some(val) = extract_number_before(&line, "eV") {
                final_enthalpy = Some(val);
            }
        }

        // 提取能量
        // "energy  without entropy=     -123.456789  energy(sigma->0) =     -123.456789"
        if line.contains("energy  without entropy") {
            if let Some(pos) = line.find("energy(sigma->0)") {
                let rest = &line[pos..];
                if let Some(val) = extract_number_after(rest, "=") {
                    final_energy = Some(val);
                }
            }
        }

        // 提取体积
        // "  volume of cell :      123.456789"
        if line.contains("volume of cell") {
            if let Some(val) = extract_last_number(&line) {
                volume = Some(val);
            }
        }

        // 提取原子数
        // "   NIONS =       8"
        if line.contains("NIONS =") {
            if let Some(val) = extract_last_number(&line) {
                num_atoms = Some(val as usize);
            }
        }
    }

    result.enthalpy_ev = final_enthalpy;
    result.energy_ev = final_energy;
    result.volume = volume;
    result.num_atoms = num_atoms;

    // 检查 CONTCAR 是否存在
    let contcar = path.parent().map(|p| p.join("CONTCAR"));
    if let Some(ref c) = contcar {
        if c.exists() {
            result.structure_file = Some(c.display().to_string());
        }
    }

    Ok(result)
}

/// 从字符串中提取指定标记之前的数字
fn extract_number_before(s: &str, marker: &str) -> Option<f64> {
    if let Some(pos) = s.find(marker) {
        let before = &s[..pos];
        before.split_whitespace().last()?.parse().ok()
    } else {
        None
    }
}

/// 从字符串中提取指定标记之后的数字
fn extract_number_after(s: &str, marker: &str) -> Option<f64> {
    if let Some(pos) = s.find(marker) {
        let after = &s[pos + marker.len()..];
        after.trim().split_whitespace().next()?.parse().ok()
    } else {
        None
    }
}

/// 提取字符串中最后一个数字
fn extract_last_number(s: &str) -> Option<f64> {
    s.split_whitespace()
        .filter_map(|w| w.parse::<f64>().ok())
        .last()
}
