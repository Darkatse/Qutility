//! # AIRSS .res 格式解析器
//!
//! 解析/写出 AIRSS (SHELX/SHLX) `.res` 结构文件格式。
//!
//! ## .res 格式说明
//! ```text
//! TITL name P V H spin modspin n (sym) n - copies
//! CELL 1.54180 a b c alpha beta gamma
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
                // AIRSS/cabal/cryan 常用 header: TITL name P V H spin modspin n (sym) n - copies
                // 兼容：若只有 4 tokens，则视为 TITL name V H (无压力)。
                let sym_start = line.find('(');
                if let (Some(start), Some(end)) = (sym_start, line.find(')')) {
                    if end > start {
                        space_group = Some(line[start + 1..end].to_string());
                    }
                }

                let header_end = sym_start
                    .or_else(|| line.find(" n -"))
                    .unwrap_or_else(|| line.len());
                let header = line[..header_end].trim();
                let tokens: Vec<&str> = header.split_whitespace().collect();

                if tokens.len() >= 2 {
                    name = tokens[1].to_string();
                }

                // 参照 cryan.f90 的宽松解析：主要关心 P/V/H 与可选 spin/modspin。
                if tokens.len() == 4 {
                    // TITL name V H
                    volume = tokens.get(2).and_then(|v| v.parse().ok());
                    enthalpy = tokens.get(3).and_then(|v| v.parse().ok());
                } else if tokens.len() >= 5 {
                    pressure = tokens.get(2).and_then(|v| v.parse().ok());
                    volume = tokens.get(3).and_then(|v| v.parse().ok());
                    enthalpy = tokens.get(4).and_then(|v| v.parse().ok());

                    // TITL name P V H spin modspin ...
                    if tokens.len() >= 7 {
                        integrated_spin = tokens.get(5).and_then(|v| v.parse().ok());
                        integrated_abs_spin = tokens.get(6).and_then(|v| v.parse().ok());
                    }
                }

                // 兼容旧的 "spin: N M" 行尾标记
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
                    let raw_label = parts[0];
                    let element = raw_label
                        .find(|c: char| c.is_ascii_digit())
                        .map(|idx| &raw_label[..idx])
                        .filter(|s| !s.is_empty())
                        .unwrap_or(raw_label);
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

    // TITL 行（对齐 AIRSS/cabal 输出，便于 cryan 等工具读取）
    let pressure = crystal.pressure.unwrap_or(0.0);
    let volume = crystal
        .volume
        .unwrap_or_else(|| crystal.lattice.volume().abs());
    let enthalpy = crystal
        .enthalpy
        .or(crystal.energy)
        .unwrap_or(0.0);
    let spin = crystal.integrated_spin.unwrap_or(0.0);
    let modspin = crystal.integrated_abs_spin.unwrap_or(0.0);
    let space_group = crystal.space_group.as_deref().unwrap_or("P1");
    let num_copies = 1;

    let mut result = format!(
        "TITL {} {:.6} {:.6} {:.10} {:.6} {:.6} {} ({}) n - {}\n",
        crystal.name, pressure, volume, enthalpy, spin, modspin, n_atoms, space_group, num_copies
    );

    // CELL 行（SHELX: 第 2 列通常为 X-ray wavelength；AIRSS/cabal 默认为 1.54180）
    result.push_str(&format!(
        "CELL 1.54180 {:.10} {:.10} {:.10} {:.6} {:.6} {:.6}\n",
        a, b, c, alpha, beta, gamma
    ));

    // LATT 行
    result.push_str("LATT -1\n");

    // SFAC 行
    result.push_str(&format!("SFAC {}\n", elements.join(" ")));

    // 原子行
    fn wrap01(x: f64) -> f64 {
        x - x.floor()
    }
    for atom in &crystal.atoms {
        let element_idx = elements
            .iter()
            .position(|e| e.eq_ignore_ascii_case(&atom.element))
            .unwrap_or(0)
            + 1;
        result.push_str(&format!(
            "{} {} {:.10} {:.10} {:.10} 1.0\n",
            atom.element,
            element_idx,
            wrap01(atom.position[0]),
            wrap01(atom.position[1]),
            wrap01(atom.position[2])
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
TITL TiC-12345 100.0 50.0 -99.5 0 0 8 (Fm-3m) n - 1
CELL 1.54180 4.33 4.33 4.33 90.0 90.0 90.0
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
TITL Fe2-123 0.0 25.0 -50.0 2.5 3.0 2 (P-1) n - 1
CELL 1.54180 2.87 2.87 2.87 90.0 90.0 90.0
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
    fn test_parse_res_allows_numbered_labels() {
        let content = r#"
TITL NaCl-1 0.0 125.0 -10.0 0 0 2 (P1) n - 1
CELL 1.54180 5.0 5.0 5.0 90.0 90.0 90.0
LATT -1
SFAC Na Cl
Na1 1 0.0 0.0 0.0 1.0
Cl2 2 0.5 0.5 0.5 1.0
END
"#;
        let crystal = parse_res_content(content, "test").unwrap();
        assert_eq!(crystal.atoms.len(), 2);
        assert_eq!(crystal.atoms[0].element, "Na");
        assert_eq!(crystal.atoms[1].element, "Cl");
    }

    #[test]
    fn test_parse_res_missing_cell() {
        let content = r#"
TITL Test 0.0 10.0 0.0 0 0 1 (P1) n - 1
SFAC Fe
Fe 1 0.0 0.0 0.0 1.0
END
"#;
        let result = parse_res_content(content, "test");
        assert!(result.is_err());
    }
}
