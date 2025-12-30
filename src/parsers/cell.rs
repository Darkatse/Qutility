//! # CASTEP .cell 格式解析器
//!
//! 解析 CASTEP 输入文件 .cell 格式。
//!
//! ## .cell 格式说明
//! ```text
//! %BLOCK LATTICE_CART
//! ang
//! a1 a2 a3
//! b1 b2 b3
//! c1 c2 c3
//! %ENDBLOCK LATTICE_CART
//!
//! %BLOCK POSITIONS_FRAC
//! Element x y z
//! ...
//! %ENDBLOCK POSITIONS_FRAC
//! ```
//!
//! ## 依赖关系
//! - 被 `parsers/mod.rs` 使用
//! - 使用 `models/structure.rs`

use crate::error::{QutilityError, Result};
use crate::models::{Atom, Crystal, Lattice};
use std::fs;
use std::path::Path;

/// 解析 .cell 文件
pub fn parse_cell_file(path: &Path) -> Result<Crystal> {
    let content = fs::read_to_string(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    parse_cell_content(
        &content,
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown"),
    )
}

/// 从字符串内容解析 .cell 格式
pub fn parse_cell_content(content: &str, default_name: &str) -> Result<Crystal> {
    let content_upper = content.to_uppercase();
    let lines: Vec<&str> = content.lines().collect();

    let mut lattice: Option<Lattice> = None;
    let mut atoms: Vec<Atom> = Vec::new();

    // 解析 LATTICE_CART 或 LATTICE_ABC
    if let Some(start) = find_block_start(&content_upper, "LATTICE_CART") {
        lattice = Some(parse_lattice_cart(&lines, start)?);
    } else if let Some(start) = find_block_start(&content_upper, "LATTICE_ABC") {
        lattice = Some(parse_lattice_abc(&lines, start)?);
    }

    // 解析 POSITIONS_FRAC 或 POSITIONS_ABS
    if let Some(start) = find_block_start(&content_upper, "POSITIONS_FRAC") {
        atoms = parse_positions(&lines, start, false)?;
    } else if let Some(start) = find_block_start(&content_upper, "POSITIONS_ABS") {
        // 对于绝对坐标，需要转换为分数坐标
        let atoms_abs = parse_positions(&lines, start, true)?;
        if let Some(ref lat) = lattice {
            atoms = convert_abs_to_frac(atoms_abs, lat);
        } else {
            atoms = atoms_abs; // 没有晶格信息时保持原样
        }
    }

    let lattice = lattice.ok_or_else(|| QutilityError::ParseError {
        format: "cell".to_string(),
        path: default_name.to_string(),
        reason: "Missing LATTICE_CART or LATTICE_ABC block".to_string(),
    })?;

    let mut crystal = Crystal::new(default_name, lattice, atoms);
    crystal.source_format = Some("cell".to_string());

    Ok(crystal)
}

/// 查找 %BLOCK XXX 的起始行号
fn find_block_start(content_upper: &str, block_name: &str) -> Option<usize> {
    let pattern = format!("%BLOCK {}", block_name);
    for (i, line) in content_upper.lines().enumerate() {
        if line.trim().starts_with(&pattern) {
            return Some(i);
        }
    }
    None
}

/// 解析 LATTICE_CART 块
fn parse_lattice_cart(lines: &[&str], start: usize) -> Result<Lattice> {
    let mut matrix = [[0.0; 3]; 3];
    let mut row_idx = 0;

    for line in lines.iter().skip(start + 1) {
        let line = line.trim();
        if line.to_uppercase().starts_with("%ENDBLOCK") {
            break;
        }
        // 跳过单位行（如 "ang" 或 "bohr"）
        if line.eq_ignore_ascii_case("ang")
            || line.eq_ignore_ascii_case("bohr")
            || line.eq_ignore_ascii_case("nm")
        {
            continue;
        }
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }

        let parts: Vec<f64> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if parts.len() >= 3 && row_idx < 3 {
            matrix[row_idx] = [parts[0], parts[1], parts[2]];
            row_idx += 1;
        }
    }

    if row_idx < 3 {
        return Err(QutilityError::ParseError {
            format: "cell".to_string(),
            path: "unknown".to_string(),
            reason: "Incomplete LATTICE_CART block".to_string(),
        });
    }

    Ok(Lattice::from_vectors(matrix))
}

/// 解析 LATTICE_ABC 块
fn parse_lattice_abc(lines: &[&str], start: usize) -> Result<Lattice> {
    let mut params: Vec<f64> = Vec::new();

    for line in lines.iter().skip(start + 1) {
        let line = line.trim();
        if line.to_uppercase().starts_with("%ENDBLOCK") {
            break;
        }
        if line.eq_ignore_ascii_case("ang")
            || line.eq_ignore_ascii_case("bohr")
            || line.is_empty()
            || line.starts_with('#')
            || line.starts_with('!')
        {
            continue;
        }

        for part in line.split_whitespace() {
            if let Ok(v) = part.parse() {
                params.push(v);
            }
        }
    }

    if params.len() < 6 {
        return Err(QutilityError::ParseError {
            format: "cell".to_string(),
            path: "unknown".to_string(),
            reason: "Incomplete LATTICE_ABC block (need a b c alpha beta gamma)".to_string(),
        });
    }

    Ok(Lattice::from_parameters(
        params[0], params[1], params[2], params[3], params[4], params[5],
    ))
}

/// 解析原子位置块
fn parse_positions(lines: &[&str], start: usize, _is_absolute: bool) -> Result<Vec<Atom>> {
    let mut atoms = Vec::new();

    for line in lines.iter().skip(start + 1) {
        let line = line.trim();
        if line.to_uppercase().starts_with("%ENDBLOCK") {
            break;
        }
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let element = parts[0].to_string();
            if let (Ok(x), Ok(y), Ok(z)) = (
                parts[1].parse::<f64>(),
                parts[2].parse::<f64>(),
                parts[3].parse::<f64>(),
            ) {
                atoms.push(Atom::new(element, [x, y, z]));
            }
        }
    }

    Ok(atoms)
}

/// 将绝对坐标转换为分数坐标
fn convert_abs_to_frac(atoms: Vec<Atom>, lattice: &Lattice) -> Vec<Atom> {
    // 计算晶格矩阵的逆矩阵
    let m = lattice.matrix;
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-10 {
        return atoms; // 奇异矩阵，返回原始
    }

    let inv = [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) / det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) / det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) / det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) / det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) / det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) / det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) / det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) / det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) / det,
        ],
    ];

    atoms
        .into_iter()
        .map(|atom| {
            let p = atom.position;
            let frac = [
                inv[0][0] * p[0] + inv[0][1] * p[1] + inv[0][2] * p[2],
                inv[1][0] * p[0] + inv[1][1] * p[1] + inv[1][2] * p[2],
                inv[2][0] * p[0] + inv[2][1] * p[1] + inv[2][2] * p[2],
            ];
            Atom::new(atom.element, frac)
        })
        .collect()
}

/// 将 Crystal 转换为 .cell 格式字符串
pub fn to_cell_string(crystal: &Crystal) -> String {
    let m = crystal.lattice.matrix;

    let mut result = String::new();

    // LATTICE_CART 块
    result.push_str("%BLOCK LATTICE_CART\nang\n");
    for row in &m {
        result.push_str(&format!(
            "{:16.10} {:16.10} {:16.10}\n",
            row[0], row[1], row[2]
        ));
    }
    result.push_str("%ENDBLOCK LATTICE_CART\n\n");

    // POSITIONS_FRAC 块
    result.push_str("%BLOCK POSITIONS_FRAC\n");
    for atom in &crystal.atoms {
        result.push_str(&format!(
            "{:4} {:16.10} {:16.10} {:16.10}\n",
            atom.element, atom.position[0], atom.position[1], atom.position[2]
        ));
    }
    result.push_str("%ENDBLOCK POSITIONS_FRAC\n");

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_lattice_cart() {
        let content = r#"
%BLOCK LATTICE_CART
ang
5.0 0.0 0.0
0.0 5.0 0.0
0.0 0.0 5.0
%ENDBLOCK LATTICE_CART

%BLOCK POSITIONS_FRAC
Na 0.0 0.0 0.0
Cl 0.5 0.5 0.5
%ENDBLOCK POSITIONS_FRAC
"#;
        let crystal = parse_cell_content(content, "NaCl").unwrap();
        assert_eq!(crystal.atoms.len(), 2);

        let (a, b, c, _, _, _) = crystal.lattice.parameters();
        assert!((a - 5.0).abs() < 0.01);
        assert!((b - 5.0).abs() < 0.01);
        assert!((c - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_cell_lattice_abc() {
        let content = r#"
%BLOCK LATTICE_ABC
ang
5.64 5.64 5.64
90.0 90.0 90.0
%ENDBLOCK LATTICE_ABC

%BLOCK POSITIONS_FRAC
Na 0.0 0.0 0.0
Cl 0.5 0.5 0.5
%ENDBLOCK POSITIONS_FRAC
"#;
        let crystal = parse_cell_content(content, "NaCl").unwrap();
        let (a, b, c, alpha, beta, gamma) = crystal.lattice.parameters();

        assert!((a - 5.64).abs() < 0.01);
        assert!((alpha - 90.0).abs() < 0.01);
        assert!((beta - 90.0).abs() < 0.01);
        assert!((gamma - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_cell_round_trip() {
        let lattice = Lattice::from_parameters(4.0, 4.0, 4.0, 90.0, 90.0, 90.0);
        let atoms = vec![
            Atom::new("Si", [0.0, 0.0, 0.0]),
            Atom::new("Si", [0.25, 0.25, 0.25]),
        ];
        let crystal = Crystal::new("Si-diamond", lattice, atoms);

        let cell_str = to_cell_string(&crystal);
        let parsed = parse_cell_content(&cell_str, "round_trip").unwrap();

        assert_eq!(parsed.atoms.len(), 2);
        assert!((parsed.atoms[0].position[0] - 0.0).abs() < 1e-6);
        assert!((parsed.atoms[1].position[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_parse_cell_with_comments() {
        let content = r#"
# This is a comment
! Another comment
%BLOCK LATTICE_CART
ang
3.0 0.0 0.0
0.0 3.0 0.0
0.0 0.0 3.0
%ENDBLOCK LATTICE_CART

%BLOCK POSITIONS_FRAC
# Fe at origin
Fe 0.0 0.0 0.0
%ENDBLOCK POSITIONS_FRAC
"#;
        let crystal = parse_cell_content(content, "Fe").unwrap();
        assert_eq!(crystal.atoms.len(), 1);
        assert_eq!(crystal.atoms[0].element, "Fe");
    }
}
