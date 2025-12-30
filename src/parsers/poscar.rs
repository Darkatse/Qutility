//! # VASP POSCAR 格式解析器
//!
//! 解析 VASP POSCAR/CONTCAR 文件格式。
//!
//! ## POSCAR 格式说明
//! ```text
//! Comment line (structure name)
//! 1.0                    # scaling factor
//! a1 a2 a3               # lattice vector a
//! b1 b2 b3               # lattice vector b
//! c1 c2 c3               # lattice vector c
//! Element1 Element2 ...  # element symbols (VASP 5+)
//! n1 n2 ...              # number of atoms per element
//! Selective dynamics     # optional
//! Direct/Cartesian       # coordinate type
//! x1 y1 z1               # atom positions
//! ...
//! ```
//!
//! ## 依赖关系
//! - 被 `parsers/mod.rs` 使用
//! - 使用 `models/structure.rs`

use crate::error::{QutilityError, Result};
use crate::models::{Atom, Crystal, Lattice};
use std::fs;
use std::path::Path;

/// 解析 POSCAR/CONTCAR 文件
pub fn parse_poscar_file(path: &Path) -> Result<Crystal> {
    let content = fs::read_to_string(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    parse_poscar_content(
        &content,
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown"),
    )
}

/// 从字符串内容解析 POSCAR 格式
pub fn parse_poscar_content(content: &str, default_name: &str) -> Result<Crystal> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() < 8 {
        return Err(QutilityError::ParseError {
            format: "poscar".to_string(),
            path: default_name.to_string(),
            reason: "File too short".to_string(),
        });
    }

    // Line 0: Comment/name
    let name = lines[0].trim().to_string();
    let name = if name.is_empty() {
        default_name.to_string()
    } else {
        name
    };

    // Line 1: Scaling factor
    let scale: f64 = lines[1].trim().parse().unwrap_or(1.0);

    // Lines 2-4: Lattice vectors
    let mut matrix = [[0.0; 3]; 3];
    for i in 0..3 {
        let parts: Vec<f64> = lines[2 + i]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() < 3 {
            return Err(QutilityError::ParseError {
                format: "poscar".to_string(),
                path: name.clone(),
                reason: format!("Invalid lattice vector at line {}", 3 + i),
            });
        }
        matrix[i] = [parts[0] * scale, parts[1] * scale, parts[2] * scale];
    }
    let lattice = Lattice::from_vectors(matrix);

    // Line 5: Element symbols (VASP 5+) or atom counts (VASP 4)
    let line5_parts: Vec<&str> = lines[5].split_whitespace().collect();
    let (elements, counts, atom_line_start) = if line5_parts[0].parse::<i32>().is_ok() {
        // VASP 4 format: no element line, only counts
        // We'll use generic element names
        let counts: Vec<usize> = line5_parts.iter().filter_map(|s| s.parse().ok()).collect();
        let elements: Vec<String> = (0..counts.len()).map(|i| format!("X{}", i + 1)).collect();
        (elements, counts, 6)
    } else {
        // VASP 5+ format: element symbols on line 5, counts on line 6
        let elements: Vec<String> = line5_parts.iter().map(|s| s.to_string()).collect();
        let counts: Vec<usize> = lines[6]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        (elements, counts, 7)
    };

    // Check for "Selective dynamics" line
    let mut coord_line = atom_line_start;
    if lines.len() > coord_line
        && lines[coord_line]
            .trim()
            .to_lowercase()
            .starts_with("selective")
    {
        coord_line += 1;
    }

    // Coordinate type line
    if lines.len() <= coord_line {
        return Err(QutilityError::ParseError {
            format: "poscar".to_string(),
            path: name.clone(),
            reason: "Missing coordinate type line".to_string(),
        });
    }

    let coord_type = lines[coord_line].trim().to_lowercase();
    let is_cartesian = coord_type.starts_with('c') || coord_type.starts_with('k');

    // Parse atom positions
    let mut atoms: Vec<Atom> = Vec::new();
    let mut line_idx = coord_line + 1;

    for (elem, &count) in elements.iter().zip(counts.iter()) {
        for _ in 0..count {
            if line_idx >= lines.len() {
                break;
            }
            let parts: Vec<f64> = lines[line_idx]
                .split_whitespace()
                .take(3)
                .filter_map(|s| s.parse().ok())
                .collect();

            if parts.len() >= 3 {
                let position = if is_cartesian {
                    // Convert Cartesian to fractional
                    cart_to_frac([parts[0], parts[1], parts[2]], &lattice)
                } else {
                    [parts[0], parts[1], parts[2]]
                };
                atoms.push(Atom::new(elem.clone(), position));
            }
            line_idx += 1;
        }
    }

    let mut crystal = Crystal::new(name, lattice, atoms);
    crystal.source_format = Some("poscar".to_string());

    Ok(crystal)
}

/// 笛卡尔坐标转分数坐标
fn cart_to_frac(cart: [f64; 3], lattice: &Lattice) -> [f64; 3] {
    let m = lattice.matrix;
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-10 {
        return cart;
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

    [
        inv[0][0] * cart[0] + inv[0][1] * cart[1] + inv[0][2] * cart[2],
        inv[1][0] * cart[0] + inv[1][1] * cart[1] + inv[1][2] * cart[2],
        inv[2][0] * cart[0] + inv[2][1] * cart[1] + inv[2][2] * cart[2],
    ]
}

/// 分数坐标转笛卡尔坐标
fn frac_to_cart(frac: [f64; 3], lattice: &Lattice) -> [f64; 3] {
    let m = lattice.matrix;
    [
        frac[0] * m[0][0] + frac[1] * m[1][0] + frac[2] * m[2][0],
        frac[0] * m[0][1] + frac[1] * m[1][1] + frac[2] * m[2][1],
        frac[0] * m[0][2] + frac[1] * m[1][2] + frac[2] * m[2][2],
    ]
}

/// 将 Crystal 转换为 POSCAR 格式字符串
pub fn to_poscar_string(crystal: &Crystal) -> String {
    use std::collections::BTreeMap;

    // 按元素分组统计
    let mut elem_order: Vec<String> = Vec::new();
    let mut elem_atoms: BTreeMap<String, Vec<[f64; 3]>> = BTreeMap::new();

    for atom in &crystal.atoms {
        if !elem_order.contains(&atom.element) {
            elem_order.push(atom.element.clone());
        }
        elem_atoms
            .entry(atom.element.clone())
            .or_default()
            .push(atom.position);
    }

    let mut result = String::new();

    // Line 0: Comment
    result.push_str(&format!("{}\n", crystal.name));

    // Line 1: Scale
    result.push_str("1.0\n");

    // Lines 2-4: Lattice
    let m = crystal.lattice.matrix;
    for row in &m {
        result.push_str(&format!(
            "  {:16.10}  {:16.10}  {:16.10}\n",
            row[0], row[1], row[2]
        ));
    }

    // Line 5: Elements
    result.push_str(&format!("   {}\n", elem_order.join("   ")));

    // Line 6: Counts
    let counts: Vec<String> = elem_order
        .iter()
        .map(|e| elem_atoms.get(e).map(|v| v.len()).unwrap_or(0).to_string())
        .collect();
    result.push_str(&format!("   {}\n", counts.join("   ")));

    // Coordinate type
    result.push_str("Direct\n");

    // Atom positions
    for elem in &elem_order {
        if let Some(positions) = elem_atoms.get(elem) {
            for pos in positions {
                result.push_str(&format!(
                    "  {:16.10}  {:16.10}  {:16.10}\n",
                    pos[0], pos[1], pos[2]
                ));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_poscar_vasp5() {
        let content = r#"NaCl
1.0
5.64 0.0 0.0
0.0 5.64 0.0
0.0 0.0 5.64
Na Cl
4 4
Direct
0.0 0.0 0.0
0.5 0.5 0.0
0.5 0.0 0.5
0.0 0.5 0.5
0.5 0.0 0.0
0.0 0.5 0.0
0.0 0.0 0.5
0.5 0.5 0.5
"#;
        let crystal = parse_poscar_content(content, "NaCl").unwrap();
        assert_eq!(crystal.name, "NaCl");
        assert_eq!(crystal.atoms.len(), 8);

        // Check element assignment
        let na_count = crystal.atoms.iter().filter(|a| a.element == "Na").count();
        let cl_count = crystal.atoms.iter().filter(|a| a.element == "Cl").count();
        assert_eq!(na_count, 4);
        assert_eq!(cl_count, 4);
    }

    #[test]
    fn test_parse_poscar_with_scale() {
        let content = r#"Si
2.0
2.0 0.0 0.0
0.0 2.0 0.0
0.0 0.0 2.0
Si
2
Direct
0.0 0.0 0.0
0.5 0.5 0.5
"#;
        let crystal = parse_poscar_content(content, "Si").unwrap();
        let (a, _, _, _, _, _) = crystal.lattice.parameters();

        // 2.0 * 2.0 = 4.0
        assert!((a - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_poscar_round_trip() {
        let lattice = Lattice::from_vectors([[4.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 4.0]]);
        let atoms = vec![
            Atom::new("Ti", [0.0, 0.0, 0.0]),
            Atom::new("O", [0.5, 0.5, 0.0]),
            Atom::new("O", [0.5, 0.0, 0.5]),
        ];
        let crystal = Crystal::new("TiO2", lattice, atoms);

        let poscar_str = to_poscar_string(&crystal);
        let parsed = parse_poscar_content(&poscar_str, "round_trip").unwrap();

        assert_eq!(parsed.atoms.len(), 3);

        let ti_count = parsed.atoms.iter().filter(|a| a.element == "Ti").count();
        let o_count = parsed.atoms.iter().filter(|a| a.element == "O").count();
        assert_eq!(ti_count, 1);
        assert_eq!(o_count, 2);
    }

    #[test]
    fn test_parse_poscar_selective_dynamics() {
        let content = r#"Fe with selective
1.0
2.87 0.0 0.0
0.0 2.87 0.0
0.0 0.0 2.87
Fe
2
Selective dynamics
Direct
0.0 0.0 0.0 T T T
0.5 0.5 0.5 F F F
"#;
        let crystal = parse_poscar_content(content, "Fe").unwrap();
        assert_eq!(crystal.atoms.len(), 2);
    }
}
