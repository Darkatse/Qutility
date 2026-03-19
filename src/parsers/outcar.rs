//! # VASP OUTCAR 解析器
//!
//! 解析 VASP OUTCAR，提取已完成输出中的物理量数据。
//!
//! ## 依赖关系
//! - 被 `dft/` 共享扫描模块调用
//! - 使用 `models/calculation.rs`

use crate::error::{QutilityError, Result};
use crate::models::{DftCodeType, DftResult};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_outcar(path: &Path, structure_name: &str) -> Result<DftResult> {
    let file = File::open(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let reader = BufReader::new(file);
    let mut result = DftResult::new(structure_name, DftCodeType::Vasp);

    let mut final_enthalpy = None;
    let mut final_energy = None;
    let mut volume = None;
    let mut num_atoms = None;

    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue,
        };

        if line.contains("enthalpy is  TOTEN") {
            if let Some(value) = extract_number_before(&line, "eV") {
                final_enthalpy = Some(value);
            }
        }

        if line.contains("energy  without entropy") {
            if let Some(pos) = line.find("energy(sigma->0)") {
                let rest = &line[pos..];
                if let Some(value) = extract_number_after(rest, "=") {
                    final_energy = Some(value);
                }
            }
        }

        if line.contains("volume of cell") {
            if let Some(value) = extract_last_number(&line) {
                volume = Some(value);
            }
        }

        if line.contains("NIONS =") {
            if let Some(value) = extract_last_number(&line) {
                num_atoms = Some(value as usize);
            }
        }
    }

    result.enthalpy_ev = final_enthalpy;
    result.energy_ev = final_energy;
    result.volume = volume;
    result.num_atoms = num_atoms;

    Ok(result)
}

fn extract_number_before(s: &str, marker: &str) -> Option<f64> {
    if let Some(pos) = s.find(marker) {
        let before = &s[..pos];
        before.split_whitespace().last()?.parse().ok()
    } else {
        None
    }
}

fn extract_number_after(s: &str, marker: &str) -> Option<f64> {
    if let Some(pos) = s.find(marker) {
        let after = &s[pos + marker.len()..];
        after.trim().split_whitespace().next()?.parse().ok()
    } else {
        None
    }
}

fn extract_last_number(s: &str) -> Option<f64> {
    s.split_whitespace()
        .filter_map(|word| word.parse::<f64>().ok())
        .last()
}
