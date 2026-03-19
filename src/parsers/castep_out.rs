//! # CASTEP .castep 解析器
//!
//! 解析 CASTEP .castep 输出，提取已完成计算的物理量数据。
//!
//! ## 依赖关系
//! - 被 `dft/` 共享扫描模块调用
//! - 使用 `models/calculation.rs`

use crate::error::{QutilityError, Result};
use crate::models::{DftCodeType, DftResult};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_castep_output(path: &Path, structure_name: &str) -> Result<DftResult> {
    let file = File::open(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let reader = BufReader::new(file);
    let mut result = DftResult::new(structure_name, DftCodeType::Castep);

    let mut final_enthalpy = None;
    let mut final_energy = None;
    let mut volume = None;
    let mut num_atoms = None;
    let mut pressure = None;

    let lines: Vec<String> = reader.lines().filter_map(|line| line.ok()).collect();

    for line in lines.iter().rev() {
        if line.contains("Final Enthalpy") && final_enthalpy.is_none() {
            if let Some(value) = extract_value_after_eq(line) {
                final_enthalpy = Some(value);
            }
        }

        if line.contains("Final energy, E") && final_energy.is_none() {
            if let Some(value) = extract_value_after_eq(line) {
                final_energy = Some(value);
            }
        }
    }

    for line in &lines {
        if line.contains("Total number of ions in cell") {
            if let Some(value) = extract_value_after_eq(line) {
                num_atoms = Some(value as usize);
            }
        }

        if line.contains("Current cell volume") {
            if let Some(value) = extract_value_after_eq(line) {
                volume = Some(value);
            }
        }

        if line.contains("Pressure:") && line.contains("GPa") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(idx) = parts.iter().position(|&token| token == "Pressure:") {
                if let Some(value) = parts
                    .get(idx + 1)
                    .and_then(|value| value.parse::<f64>().ok())
                {
                    pressure = Some(value * 10.0);
                }
            }
        }
    }

    result.enthalpy_ev = final_enthalpy;
    result.energy_ev = final_energy;
    result.volume = volume;
    result.num_atoms = num_atoms;
    result.pressure_kbar = pressure;

    Ok(result)
}

fn extract_value_after_eq(s: &str) -> Option<f64> {
    if let Some(pos) = s.find('=') {
        let after = &s[pos + 1..];
        after.trim().split_whitespace().next()?.parse().ok()
    } else {
        None
    }
}
