//! # AIRSS .res 格式解析器
//!
//! 解析 AIRSS 结构搜索产生的 .res 文件格式。
//!
//! ## .res 格式说明
//! ```text
//! TITL name P V E H 0 0 n (sym) [spin info]
//! CELL 1.0 a b c alpha beta gamma
//! LATT -1
//! SFAC Element1 Element2 ...
//! Element1 1 x1 y1 z1 1.0
//! Element2 2 x2 y2 z2 1.0
//! ...
//! END
//! ```
//!
//! ## 依赖关系
//! - 被 `parsers/mod.rs` 使用
//! - 使用 `models/structure.rs`

use crate::error::{QutilityError, Result};
use crate::models::{Atom, Crystal, Lattice};
use std::fs;
use std::path::Path;

/// 解析 .res 文件
pub fn parse_res_file(path: &Path) -> Result<Crystal> {
    let content = fs::read_to_string(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    parse_res_content(
        &content,
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown"),
    )
}

/// 从字符串内容解析 .res 格式
pub fn parse_res_content(content: &str, default_name: &str) -> Result<Crystal> {
    let mut name = default_name.to_string();
    let mut lattice: Option<Lattice> = None;
    let mut atoms: Vec<Atom> = Vec::new();
    let mut sfac_elements: Vec<String> = Vec::new();

    // TITL 行元数据
    let mut pressure: Option<f64> = None;
    let mut volume: Option<f64> = None;
    let mut enthalpy: Option<f64> = None;
    let mut space_group: Option<String> = None;
    let mut integrated_spin: Option<f64> = None;
    let mut integrated_abs_spin: Option<f64> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0].to_uppercase().as_str() {
            "TITL" => {
                // TITL name P V E H 0 0 n (sym) [spin]
                if parts.len() >= 2 {
                    name = parts[1].to_string();
                }
                if parts.len() >= 3 {
                    pressure = parts[2].parse().ok();
                }
                if parts.len() >= 4 {
                    volume = parts[3].parse().ok();
                }
                // parts[4] 是能量 E，parts[5] 是焓 H
                if parts.len() >= 6 {
                    enthalpy = parts[5].parse().ok();
                }
                // 查找 (sym) 空间群
                if let Some(sym_start) = line.find('(') {
                    if let Some(sym_end) = line.find(')') {
                        if sym_end > sym_start {
                            space_group = Some(line[sym_start + 1..sym_end].to_string());
                        }
                    }
                }
                // 查找 spin: N M 格式（在行尾）
                if let Some(spin_pos) = line.find("spin:") {
                    let spin_parts: Vec<&str> = line[spin_pos + 5..].split_whitespace().collect();
                    if spin_parts.len() >= 1 {
                        integrated_spin = spin_parts[0].parse().ok();
                    }
                    if spin_parts.len() >= 2 {
                        integrated_abs_spin = spin_parts[1].parse().ok();
                    }
                }
            }
            "CELL" => {
                // CELL scale a b c alpha beta gamma
                if parts.len() >= 8 {
                    let a: f64 = parts[2].parse().unwrap_or(1.0);
                    let b: f64 = parts[3].parse().unwrap_or(1.0);
                    let c: f64 = parts[4].parse().unwrap_or(1.0);
                    let alpha: f64 = parts[5].parse().unwrap_or(90.0);
                    let beta: f64 = parts[6].parse().unwrap_or(90.0);
                    let gamma: f64 = parts[7].parse().unwrap_or(90.0);
                    lattice = Some(Lattice::from_parameters(a, b, c, alpha, beta, gamma));
                }
            }
            "SFAC" => {
                // SFAC Element1 Element2 ...
                sfac_elements = parts[1..].iter().map(|s| s.to_string()).collect();
            }
            "LATT" | "ZERR" | "END" | "REM" => {
                // 忽略这些行
            }
            _ => {
                // 可能是原子行: Element type x y z occ
                // 原子行以元素名开头
                if parts.len() >= 5 && !sfac_elements.is_empty() {
                    let element = parts[0];
                    // 验证是否是已知元素
                    if sfac_elements
                        .iter()
                        .any(|e| e.eq_ignore_ascii_case(element))
                    {
                        if let (Ok(x), Ok(y), Ok(z)) = (
                            parts[2].parse::<f64>(),
                            parts[3].parse::<f64>(),
                            parts[4].parse::<f64>(),
                        ) {
                            atoms.push(Atom::new(element, [x, y, z]));
                        }
                    }
                }
            }
        }
    }

    let lattice = lattice.ok_or_else(|| QutilityError::ParseError {
        format: "res".to_string(),
        path: name.clone(),
        reason: "Missing CELL line".to_string(),
    })?;

    let mut crystal = Crystal::new(name, lattice, atoms);
    crystal.pressure = pressure;
    crystal.volume = volume;
    crystal.enthalpy = enthalpy;
    crystal.space_group = space_group;
    crystal.integrated_spin = integrated_spin;
    crystal.integrated_abs_spin = integrated_abs_spin;
    crystal.source_format = Some("res".to_string());

    Ok(crystal)
}

/// 将 Crystal 转换为 .res 格式字符串
pub fn to_res_string(crystal: &Crystal) -> String {
    let (a, b, c, alpha, beta, gamma) = crystal.lattice.parameters();
    let n_atoms = crystal.atoms.len();

    // 收集唯一元素
    let mut elements: Vec<String> = Vec::new();
    for atom in &crystal.atoms {
        if !elements
            .iter()
            .any(|e| e.eq_ignore_ascii_case(&atom.element))
        {
            elements.push(atom.element.clone());
        }
    }

    // TITL 行
    let pressure = crystal.pressure.unwrap_or(0.0);
    let volume = crystal
        .volume
        .unwrap_or_else(|| crystal.lattice.volume().abs());
    let energy = crystal.energy.unwrap_or(0.0);
    let enthalpy = crystal.enthalpy.unwrap_or(energy);
    let space_group = crystal.space_group.as_deref().unwrap_or("P1");

    let mut result = format!(
        "TITL {} {:.6} {:.6} {:.10} {:.10} 0 0 {} ({})",
        crystal.name, pressure, volume, energy, enthalpy, n_atoms, space_group
    );

    // 添加 spin 信息（如果有）
    if let (Some(spin), Some(abs_spin)) = (crystal.integrated_spin, crystal.integrated_abs_spin) {
        result.push_str(&format!(" spin: {:.6} {:.6}", spin, abs_spin));
    }

    result.push('\n');

    // CELL 行
    result.push_str(&format!(
        "CELL 1.0 {:.10} {:.10} {:.10} {:.6} {:.6} {:.6}\n",
        a, b, c, alpha, beta, gamma
    ));

    // LATT 行
    result.push_str("LATT -1\n");

    // SFAC 行
    result.push_str(&format!("SFAC {}\n", elements.join(" ")));

    // 原子行
    for atom in &crystal.atoms {
        let element_idx = elements
            .iter()
            .position(|e| e.eq_ignore_ascii_case(&atom.element))
            .unwrap_or(0)
            + 1;
        result.push_str(&format!(
            "{} {} {:.10} {:.10} {:.10} 1.0\n",
            atom.element, element_idx, atom.position[0], atom.position[1], atom.position[2]
        ));
    }

    result.push_str("END\n");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_res_basic() {
        let content = r#"
TITL TiC-12345 100.0 50.0 -100.0 -99.5 0 0 8 (Fm-3m)
CELL 1.0 4.33 4.33 4.33 90.0 90.0 90.0
LATT -1
SFAC Ti C
Ti 1 0.0 0.0 0.0 1.0
Ti 1 0.5 0.5 0.0 1.0
Ti 1 0.5 0.0 0.5 1.0
Ti 1 0.0 0.5 0.5 1.0
C 2 0.5 0.5 0.5 1.0
C 2 0.0 0.0 0.5 1.0
C 2 0.0 0.5 0.0 1.0
C 2 0.5 0.0 0.0 1.0
END
"#;
        let crystal = parse_res_content(content, "test").unwrap();
        assert_eq!(crystal.name, "TiC-12345");
        assert_eq!(crystal.atoms.len(), 8);
        assert_eq!(crystal.pressure, Some(100.0));
        assert_eq!(crystal.space_group, Some("Fm-3m".to_string()));
    }

    #[test]
    fn test_parse_res_with_spin() {
        let content = r#"
TITL Fe2-123 0.0 25.0 -50.0 -50.0 0 0 2 (P-1) spin: 2.5 3.0
CELL 1.0 2.87 2.87 2.87 90.0 90.0 90.0
LATT -1
SFAC Fe
Fe 1 0.0 0.0 0.0 1.0
Fe 1 0.5 0.5 0.5 1.0
END
"#;
        let crystal = parse_res_content(content, "test").unwrap();
        assert_eq!(crystal.atoms.len(), 2);
        assert_eq!(crystal.integrated_spin, Some(2.5));
        assert_eq!(crystal.integrated_abs_spin, Some(3.0));
    }

    #[test]
    fn test_res_round_trip() {
        let lattice = Lattice::from_parameters(5.0, 5.0, 5.0, 90.0, 90.0, 90.0);
        let atoms = vec![
            Atom::new("Na", [0.0, 0.0, 0.0]),
            Atom::new("Cl", [0.5, 0.5, 0.5]),
        ];
        let mut crystal = Crystal::new("NaCl-test", lattice, atoms);
        crystal.pressure = Some(0.0);
        crystal.enthalpy = Some(-100.5);
        crystal.space_group = Some("Fm-3m".to_string());

        // Convert to string and back
        let res_str = to_res_string(&crystal);
        let parsed = parse_res_content(&res_str, "round_trip").unwrap();

        assert_eq!(parsed.name, "NaCl-test");
        assert_eq!(parsed.atoms.len(), 2);
        assert_eq!(parsed.space_group, Some("Fm-3m".to_string()));

        // Check atom positions match
        assert!((parsed.atoms[0].position[0] - 0.0).abs() < 1e-6);
        assert!((parsed.atoms[1].position[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_parse_res_missing_cell() {
        let content = r#"
TITL Test 0.0 10.0 0.0 0.0 0 0 1 (P1)
SFAC Fe
Fe 1 0.0 0.0 0.0 1.0
END
"#;
        let result = parse_res_content(content, "test");
        assert!(result.is_err());
    }
}
