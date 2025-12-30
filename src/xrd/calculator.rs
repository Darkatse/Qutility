//! # XRD 衍射峰计算器
//!
//! 实现 X 射线衍射图样计算的核心算法。
//!
//! ## 算法概述
//! 1. 计算倒格矢
//! 2. 遍历限制球内的 (hkl) 点
//! 3. 计算 Bragg 角和 d 间距
//! 4. 计算结构因子
//! 5. 应用 Lorentz 极化校正
//!
//! ## 参考
//! - pymatgen.analysis.diffraction.xrd
//! - Structure of Materials by Marc De Graef and Michael E. McHenry
//!
//! ## 依赖关系
//! - 被 `commands/analyze/xrd.rs` 调用
//! - 使用 `models/structure.rs` 的 Crystal, Lattice 结构
//! - 使用 `xrd/scattering.rs` 获取原子散射因子

use crate::error::{QutilityError, Result};
use crate::models::{Crystal, Lattice};
use crate::xrd::scattering;

use std::f64::consts::PI;

/// 衍射峰
#[derive(Debug, Clone)]
pub struct Peak {
    /// 衍射角 2θ（度）
    pub two_theta: f64,
    /// d 间距（Å）
    pub d_spacing: f64,
    /// 相对强度（0-100）
    pub intensity: f64,
    /// Miller 指数 h
    pub h: i32,
    /// Miller 指数 k
    pub k: i32,
    /// Miller 指数 l
    pub l: i32,
}

/// XRD 衍射图谱
#[derive(Debug, Clone)]
pub struct XrdPattern {
    /// 衍射峰列表（按强度降序排列）
    pub peaks: Vec<Peak>,
    /// 使用的波长（Å）
    pub wavelength: f64,
    /// 结构名称
    pub structure_name: String,
}

/// XRD 计算器
pub struct XrdCalculator {
    /// X 射线波长（Å）
    wavelength: f64,
}

impl XrdCalculator {
    /// 创建新的 XRD 计算器
    pub fn new(wavelength: f64) -> Self {
        Self { wavelength }
    }

    /// 计算 XRD 衍射图谱
    pub fn calculate(
        &self,
        crystal: &Crystal,
        two_theta_min: f64,
        two_theta_max: f64,
    ) -> Result<XrdPattern> {
        // 检查波长
        if self.wavelength <= 0.0 {
            return Err(QutilityError::Other("Invalid wavelength".to_string()));
        }

        // 计算倒格矢矩阵
        let recip_lattice = self.reciprocal_lattice(&crystal.lattice);

        // 计算限制球半径: |G| <= 2/λ 对应 2θ = 180°
        // 对于给定的 2θ_max，有 sin(θ_max) = λ|G|/2
        let theta_max_rad = two_theta_max.to_radians() / 2.0;
        let g_max = 2.0 * theta_max_rad.sin() / self.wavelength;

        // 确定搜索范围（保守估计）
        let lattice_params = crystal.lattice.parameters();
        let max_hkl =
            ((g_max * lattice_params.0.max(lattice_params.1.max(lattice_params.2))) as i32 + 1)
                .min(30);

        // 收集所有衍射峰
        let mut peaks = Vec::new();

        for h in -max_hkl..=max_hkl {
            for k in -max_hkl..=max_hkl {
                for l in -max_hkl..=max_hkl {
                    // 跳过 (0,0,0)
                    if h == 0 && k == 0 && l == 0 {
                        continue;
                    }

                    // 计算倒格矢 G = h*b1 + k*b2 + l*b3
                    let g = self.calculate_g(&recip_lattice, h, k, l);
                    let g_mag = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();

                    if g_mag < 1e-10 {
                        continue;
                    }

                    // 计算 d 间距: d = 2π/|G| (因为 G 已经包含 2π 因子)
                    let d = 2.0 * PI / g_mag;

                    // 计算 sin(θ) = λ/(2d)
                    let sin_theta = self.wavelength / (2.0 * d);

                    // 检查是否在有效范围内
                    if sin_theta.abs() > 1.0 {
                        continue;
                    }

                    let theta = sin_theta.asin();
                    let two_theta = 2.0 * theta.to_degrees();

                    // 检查 2θ 范围
                    if two_theta < two_theta_min || two_theta > two_theta_max {
                        continue;
                    }

                    // 计算结构因子
                    let (f_real, f_imag) = self.calculate_structure_factor(crystal, &g, sin_theta);
                    let f_sq = f_real * f_real + f_imag * f_imag;

                    // 跳过强度太低的峰
                    if f_sq < 1e-10 {
                        continue;
                    }

                    // Lorentz 极化校正
                    let lp = self.lorentz_polarization(theta);

                    // 强度
                    let intensity = f_sq * lp;

                    peaks.push(Peak {
                        two_theta,
                        d_spacing: d,
                        intensity,
                        h,
                        k,
                        l,
                    });
                }
            }
        }

        // 合并等效峰（相同 2θ 的峰）
        let peaks = self.merge_equivalent_peaks(peaks);

        // 按强度降序排序
        let mut peaks = peaks;
        peaks.sort_by(|a, b| b.intensity.partial_cmp(&a.intensity).unwrap());

        // 归一化强度到 0-100
        if let Some(max_i) = peaks.first().map(|p| p.intensity) {
            if max_i > 0.0 {
                for p in &mut peaks {
                    p.intensity = 100.0 * p.intensity / max_i;
                }
            }
        }

        Ok(XrdPattern {
            peaks,
            wavelength: self.wavelength,
            structure_name: crystal.name.clone(),
        })
    }

    /// 计算倒格矢矩阵
    fn reciprocal_lattice(&self, lattice: &Lattice) -> [[f64; 3]; 3] {
        let m = lattice.matrix;

        // a, b, c 向量
        let a = [m[0][0], m[0][1], m[0][2]];
        let b = [m[1][0], m[1][1], m[1][2]];
        let c = [m[2][0], m[2][1], m[2][2]];

        // 体积 V = a · (b × c)
        let b_cross_c = cross(&b, &c);
        let volume = dot(&a, &b_cross_c);

        if volume.abs() < 1e-10 {
            return [[0.0; 3]; 3];
        }

        // 倒格矢：b1 = 2π(b×c)/V, b2 = 2π(c×a)/V, b3 = 2π(a×b)/V
        let c_cross_a = cross(&c, &a);
        let a_cross_b = cross(&a, &b);

        let factor = 2.0 * PI / volume;

        [
            [
                b_cross_c[0] * factor,
                b_cross_c[1] * factor,
                b_cross_c[2] * factor,
            ],
            [
                c_cross_a[0] * factor,
                c_cross_a[1] * factor,
                c_cross_a[2] * factor,
            ],
            [
                a_cross_b[0] * factor,
                a_cross_b[1] * factor,
                a_cross_b[2] * factor,
            ],
        ]
    }

    /// 计算倒格矢 G
    fn calculate_g(&self, recip: &[[f64; 3]; 3], h: i32, k: i32, l: i32) -> [f64; 3] {
        let hf = h as f64;
        let kf = k as f64;
        let lf = l as f64;

        [
            hf * recip[0][0] + kf * recip[1][0] + lf * recip[2][0],
            hf * recip[0][1] + kf * recip[1][1] + lf * recip[2][1],
            hf * recip[0][2] + kf * recip[1][2] + lf * recip[2][2],
        ]
    }

    /// 计算结构因子
    fn calculate_structure_factor(
        &self,
        crystal: &Crystal,
        g: &[f64; 3],
        sin_theta: f64,
    ) -> (f64, f64) {
        let s = sin_theta / self.wavelength;
        let mut f_real = 0.0;
        let mut f_imag = 0.0;

        for atom in &crystal.atoms {
            // 获取原子散射因子
            let f_atom = scattering::calculate_scattering_factor(&atom.element, s);

            // 计算相位 φ = 2π(G · r) = 2π(hx + ky + lz)
            // 但这里 G 已经乘了 2π，所以直接用 G · r
            let r = [atom.position[0], atom.position[1], atom.position[2]];

            // 需要转换：G (倒空间) · r (分数坐标)
            // G = h*b1 + k*b2 + l*b3
            // r = x*a1 + y*a2 + z*a3 (分数坐标表示)
            // G · r = h*x + k*y + l*z (因为 bi · aj = 2π δij)
            // 但这里 G 已经计算过了，所以实际上需要计算笛卡尔坐标的点积再除以 2π
            // 简化：直接用分数坐标计算相位

            // 从 G 向量反推 hkl（通过倒格矢）比较复杂
            // 简化方法：假设 g 已经正确，我们直接使用笛卡尔坐标
            let cart_r = frac_to_cart(&r, &crystal.lattice.matrix);
            let phase = g[0] * cart_r[0] + g[1] * cart_r[1] + g[2] * cart_r[2];

            f_real += f_atom * phase.cos();
            f_imag += f_atom * phase.sin();
        }

        (f_real, f_imag)
    }

    /// Lorentz 极化校正
    fn lorentz_polarization(&self, theta: f64) -> f64 {
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let cos_2theta = (2.0 * theta).cos();

        if sin_theta.abs() < 1e-10 || cos_theta.abs() < 1e-10 {
            return 0.0;
        }

        (1.0 + cos_2theta * cos_2theta) / (sin_theta * sin_theta * cos_theta)
    }

    /// 合并等效峰
    fn merge_equivalent_peaks(&self, peaks: Vec<Peak>) -> Vec<Peak> {
        let mut merged: Vec<Peak> = Vec::new();
        let tolerance = 0.01; // 2θ 容差（度）

        for peak in peaks {
            let mut found = false;
            for existing in &mut merged {
                if (existing.two_theta - peak.two_theta).abs() < tolerance {
                    // 累加强度，保留较简单的 hkl
                    existing.intensity += peak.intensity;
                    found = true;
                    break;
                }
            }
            if !found {
                merged.push(peak);
            }
        }

        merged
    }
}

/// 向量叉积
fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// 向量点积
fn dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// 分数坐标转笛卡尔坐标
fn frac_to_cart(frac: &[f64; 3], matrix: &[[f64; 3]; 3]) -> [f64; 3] {
    [
        frac[0] * matrix[0][0] + frac[1] * matrix[1][0] + frac[2] * matrix[2][0],
        frac[0] * matrix[0][1] + frac[1] * matrix[1][1] + frac[2] * matrix[2][1],
        frac[0] * matrix[0][2] + frac[1] * matrix[1][2] + frac[2] * matrix[2][2],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Atom;

    #[test]
    fn test_xrd_nacl() {
        // 创建 NaCl 结构（岩盐结构）
        let a = 5.64; // Å
        let lattice = Lattice::from_vectors([[a, 0.0, 0.0], [0.0, a, 0.0], [0.0, 0.0, a]]);

        let crystal = Crystal {
            name: "NaCl".to_string(),
            lattice,
            atoms: vec![
                Atom::new("Na", [0.0, 0.0, 0.0]),
                Atom::new("Na", [0.5, 0.5, 0.0]),
                Atom::new("Na", [0.5, 0.0, 0.5]),
                Atom::new("Na", [0.0, 0.5, 0.5]),
                Atom::new("Cl", [0.5, 0.0, 0.0]),
                Atom::new("Cl", [0.0, 0.5, 0.0]),
                Atom::new("Cl", [0.0, 0.0, 0.5]),
                Atom::new("Cl", [0.5, 0.5, 0.5]),
            ],
            pressure: None,
            enthalpy: None,
            energy: None,
            volume: Some(a * a * a),
            space_group: Some("Fm-3m".to_string()),
            integrated_spin: None,
            integrated_abs_spin: None,
            source_format: None,
        };

        let calc = XrdCalculator::new(1.5418); // Cu Kα
        let pattern = calc.calculate(&crystal, 10.0, 90.0).unwrap();

        assert!(!pattern.peaks.is_empty(), "Should have diffraction peaks");
        println!("Found {} peaks for NaCl", pattern.peaks.len());
        for p in pattern.peaks.iter().take(5) {
            println!(
                "2θ = {:.2}°, d = {:.4} Å, I = {:.1}%, ({} {} {})",
                p.two_theta, p.d_spacing, p.intensity, p.h, p.k, p.l
            );
        }
    }
}
