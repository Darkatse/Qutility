//! # 晶体结构数据模型
//!
//! 定义统一的晶体结构表示，可以从不同格式解析并转换为不同格式。
//!
//! ## 依赖关系
//! - 被 `parsers/` 和 `converters/` 使用
//! - 无外部模块依赖

use serde::{Deserialize, Serialize};

/// 晶格参数表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lattice {
    /// 晶格向量矩阵 (3x3)，行向量表示 a, b, c
    /// [[a1, a2, a3], [b1, b2, b3], [c1, c2, c3]]
    pub matrix: [[f64; 3]; 3],
}

impl Lattice {
    /// 从晶格参数 (a, b, c, alpha, beta, gamma) 创建晶格
    /// 角度单位：度
    pub fn from_parameters(a: f64, b: f64, c: f64, alpha: f64, beta: f64, gamma: f64) -> Self {
        let alpha_rad = alpha.to_radians();
        let beta_rad = beta.to_radians();
        let gamma_rad = gamma.to_radians();

        // 计算晶格向量
        let cos_alpha = alpha_rad.cos();
        let cos_beta = beta_rad.cos();
        let cos_gamma = gamma_rad.cos();
        let sin_gamma = gamma_rad.sin();

        let a_vec = [a, 0.0, 0.0];
        let b_vec = [b * cos_gamma, b * sin_gamma, 0.0];

        let c1 = c * cos_beta;
        let c2 = c * (cos_alpha - cos_beta * cos_gamma) / sin_gamma;
        let c3 = (c * c - c1 * c1 - c2 * c2).sqrt();
        let c_vec = [c1, c2, c3];

        Lattice {
            matrix: [a_vec, b_vec, c_vec],
        }
    }

    /// 从晶格向量矩阵创建
    pub fn from_vectors(matrix: [[f64; 3]; 3]) -> Self {
        Lattice { matrix }
    }

    /// 获取晶格参数 (a, b, c, alpha, beta, gamma)
    pub fn parameters(&self) -> (f64, f64, f64, f64, f64, f64) {
        let a_vec = self.matrix[0];
        let b_vec = self.matrix[1];
        let c_vec = self.matrix[2];

        let a = (a_vec[0].powi(2) + a_vec[1].powi(2) + a_vec[2].powi(2)).sqrt();
        let b = (b_vec[0].powi(2) + b_vec[1].powi(2) + b_vec[2].powi(2)).sqrt();
        let c = (c_vec[0].powi(2) + c_vec[1].powi(2) + c_vec[2].powi(2)).sqrt();

        let dot_bc: f64 = b_vec.iter().zip(c_vec.iter()).map(|(x, y)| x * y).sum();
        let dot_ac: f64 = a_vec.iter().zip(c_vec.iter()).map(|(x, y)| x * y).sum();
        let dot_ab: f64 = a_vec.iter().zip(b_vec.iter()).map(|(x, y)| x * y).sum();

        let alpha = (dot_bc / (b * c)).acos().to_degrees();
        let beta = (dot_ac / (a * c)).acos().to_degrees();
        let gamma = (dot_ab / (a * b)).acos().to_degrees();

        (a, b, c, alpha, beta, gamma)
    }

    /// 计算晶格体积
    pub fn volume(&self) -> f64 {
        let a = self.matrix[0];
        let b = self.matrix[1];
        let c = self.matrix[2];

        // 行列式计算
        a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0])
    }
}

/// 原子信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    /// 元素符号
    pub element: String,

    /// 分数坐标 [x, y, z]
    pub position: [f64; 3],

    /// 可选：原子标签（用于区分同种元素的不同位置）
    pub label: Option<String>,
}

impl Atom {
    pub fn new(element: impl Into<String>, position: [f64; 3]) -> Self {
        Atom {
            element: element.into(),
            position,
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// 晶体结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crystal {
    /// 结构名称
    pub name: String,

    /// 晶格
    pub lattice: Lattice,

    /// 原子列表
    pub atoms: Vec<Atom>,

    /// 压力 (GPa)
    pub pressure: Option<f64>,

    /// 焓 (eV)
    pub enthalpy: Option<f64>,

    /// 能量 (eV)
    pub energy: Option<f64>,

    /// 体积 (Å³)
    pub volume: Option<f64>,

    /// 空间群
    pub space_group: Option<String>,

    /// 每原子积分自旋 (AIRSS .res 特有)
    pub integrated_spin: Option<f64>,

    /// 每原子绝对积分自旋 (AIRSS .res 特有)
    pub integrated_abs_spin: Option<f64>,

    /// 来源文件格式
    pub source_format: Option<String>,
}

impl Crystal {
    pub fn new(name: impl Into<String>, lattice: Lattice, atoms: Vec<Atom>) -> Self {
        Crystal {
            name: name.into(),
            lattice,
            atoms,
            pressure: None,
            enthalpy: None,
            energy: None,
            volume: None,
            space_group: None,
            integrated_spin: None,
            integrated_abs_spin: None,
            source_format: None,
        }
    }

    /// 计算化学式
    pub fn formula(&self) -> String {
        use std::collections::BTreeMap;
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();

        for atom in &self.atoms {
            *counts.entry(atom.element.as_str()).or_insert(0) += 1;
        }

        counts
            .into_iter()
            .map(|(el, count)| {
                if count == 1 {
                    el.to_string()
                } else {
                    format!("{}{}", el, count)
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// 计算每原子焓
    pub fn enthalpy_per_atom(&self) -> Option<f64> {
        self.enthalpy.map(|h| h / self.atoms.len() as f64)
    }

    /// 计算每原子体积
    pub fn volume_per_atom(&self) -> Option<f64> {
        let vol = self.volume.unwrap_or_else(|| self.lattice.volume().abs());
        Some(vol / self.atoms.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lattice_from_parameters_cubic() {
        let lattice = Lattice::from_parameters(5.0, 5.0, 5.0, 90.0, 90.0, 90.0);
        let (a, b, c, alpha, beta, gamma) = lattice.parameters();

        assert!((a - 5.0).abs() < 1e-6);
        assert!((b - 5.0).abs() < 1e-6);
        assert!((c - 5.0).abs() < 1e-6);
        assert!((alpha - 90.0).abs() < 1e-6);
        assert!((beta - 90.0).abs() < 1e-6);
        assert!((gamma - 90.0).abs() < 1e-6);
    }

    #[test]
    fn test_lattice_volume_cubic() {
        let lattice = Lattice::from_parameters(5.0, 5.0, 5.0, 90.0, 90.0, 90.0);
        let vol = lattice.volume().abs();

        // 5^3 = 125
        assert!((vol - 125.0).abs() < 1e-6);
    }

    #[test]
    fn test_lattice_from_vectors() {
        let lattice = Lattice::from_vectors([[4.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 4.0]]);
        let (a, b, c, _, _, _) = lattice.parameters();

        assert!((a - 4.0).abs() < 1e-6);
        assert!((b - 4.0).abs() < 1e-6);
        assert!((c - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_lattice_hexagonal() {
        let lattice = Lattice::from_parameters(3.0, 3.0, 5.0, 90.0, 90.0, 120.0);
        let (a, b, c, alpha, beta, gamma) = lattice.parameters();

        assert!((a - 3.0).abs() < 0.01);
        assert!((b - 3.0).abs() < 0.01);
        assert!((c - 5.0).abs() < 0.01);
        assert!((gamma - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_crystal_formula() {
        let lattice = Lattice::from_parameters(5.0, 5.0, 5.0, 90.0, 90.0, 90.0);
        let atoms = vec![
            Atom::new("Na", [0.0, 0.0, 0.0]),
            Atom::new("Na", [0.5, 0.5, 0.0]),
            Atom::new("Na", [0.5, 0.0, 0.5]),
            Atom::new("Na", [0.0, 0.5, 0.5]),
            Atom::new("Cl", [0.5, 0.0, 0.0]),
            Atom::new("Cl", [0.0, 0.5, 0.0]),
            Atom::new("Cl", [0.0, 0.0, 0.5]),
            Atom::new("Cl", [0.5, 0.5, 0.5]),
        ];
        let crystal = Crystal::new("NaCl", lattice, atoms);

        let formula = crystal.formula();
        assert!(formula.contains("Cl"));
        assert!(formula.contains("Na"));
    }

    #[test]
    fn test_crystal_enthalpy_per_atom() {
        let lattice = Lattice::from_parameters(5.0, 5.0, 5.0, 90.0, 90.0, 90.0);
        let atoms = vec![
            Atom::new("Fe", [0.0, 0.0, 0.0]),
            Atom::new("Fe", [0.5, 0.5, 0.5]),
        ];
        let mut crystal = Crystal::new("Fe", lattice, atoms);
        crystal.enthalpy = Some(-20.0);

        let h_per_atom = crystal.enthalpy_per_atom().unwrap();
        assert!((h_per_atom - (-10.0)).abs() < 1e-6);
    }

    #[test]
    fn test_atom_with_label() {
        let atom = Atom::new("Fe", [0.0, 0.0, 0.0]).with_label("Fe1");
        assert_eq!(atom.label, Some("Fe1".to_string()));
    }
}
