//! # CASTEP .castep 输出解析器
//!
//! 解析 CASTEP 计算输出文件 .castep，提取焓、能量等信息。
//!
//! ## 依赖关系
//! - 被 `commands/analyze.rs`, `commands/collect.rs` 使用
//! - 使用 `models/calculation.rs`

use crate::error::{QutilityError, Result};
use crate::models::{DftCodeType, DftResult};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// 解析 CASTEP .castep 输出文件
pub fn parse_castep_output(path: &Path, structure_name: &str) -> Result<DftResult> {
    let file = File::open(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let reader = BufReader::new(file);
    let mut result = DftResult::new(structure_name, DftCodeType::Castep);

    let mut final_enthalpy: Option<f64> = None;
    let mut final_energy: Option<f64> = None;
    let mut volume: Option<f64> = None;
    let mut num_atoms: Option<usize> = None;
    let mut pressure: Option<f64> = None;

    // 读取所有行（需要从后往前查找某些内容）
    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    // 从后往前查找完成标志和最终值
    for line in lines.iter().rev() {
        // 检查是否完成
        if line.contains("Total time") {
            result.is_finished = true;
        }

        // 提取最终焓
        // "Final Enthalpy     =   -1234.56789012     eV"
        if line.contains("Final Enthalpy") && final_enthalpy.is_none() {
            if let Some(val) = extract_value_after_eq(line) {
                final_enthalpy = Some(val);
            }
        }

        // 提取最终能量
        // "Final energy, E             =  -1234.56789012     eV"
        if line.contains("Final energy, E") && final_energy.is_none() {
            if let Some(val) = extract_value_after_eq(line) {
                final_energy = Some(val);
            }
        }
    }

    // 从前往后查找其他信息
    for line in &lines {
        // 提取原子数
        // "Total number of ions in cell =       8"
        if line.contains("Total number of ions in cell") {
            if let Some(val) = extract_value_after_eq(line) {
                num_atoms = Some(val as usize);
            }
        }

        // 提取单元体积
        // "Current cell volume =           123.456789       A**3"
        if line.contains("Current cell volume") {
            if let Some(val) = extract_value_after_eq(line) {
                volume = Some(val);
            }
        }

        // 提取压力
        // "       Pressure:     100.0000      GPa"
        if line.contains("Pressure:") && line.contains("GPa") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(idx) = parts.iter().position(|&s| s == "Pressure:") {
                if idx + 1 < parts.len() {
                    if let Ok(p) = parts[idx + 1].parse::<f64>() {
                        pressure = Some(p * 10.0); // GPa -> kBar
                    }
                }
            }
        }
    }

    result.enthalpy_ev = final_enthalpy;
    result.energy_ev = final_energy;
    result.volume = volume;
    result.num_atoms = num_atoms;
    result.pressure_kbar = pressure;

    // 检查 -out.cell 或同名 .cell 是否存在
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if let Some(parent) = path.parent() {
        // 优先 -out.cell（优化后的结构）
        let out_cell = parent.join(format!("{}-out.cell", stem));
        if out_cell.exists() {
            result.structure_file = Some(out_cell.display().to_string());
        } else {
            // 否则使用输入 .cell
            let cell = parent.join(format!("{}.cell", stem));
            if cell.exists() {
                result.structure_file = Some(cell.display().to_string());
            }
        }
    }

    Ok(result)
}

/// 提取等号后的数值
fn extract_value_after_eq(s: &str) -> Option<f64> {
    if let Some(pos) = s.find('=') {
        let after = &s[pos + 1..];
        after.trim().split_whitespace().next()?.parse().ok()
    } else {
        None
    }
}
