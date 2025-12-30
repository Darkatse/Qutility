//! # 原子散射因子数据库
//!
//! 提供原子 X 射线散射因子的计算。
//!
//! ## 公式
//! f(s) = Σᵢ aᵢ exp(-bᵢ s²) + c
//! 其中 s = sin(θ)/λ
//!
//! ## 数据来源
//! International Tables for Crystallography, Vol. C, Table 6.1.1.4
//! http://it.iucr.org/Cb/ch6o1v0001/
//!
//! ## 依赖关系
//! - 被 `xrd/calculator.rs` 调用计算原子散射因子
//! - 纯静态数据，无外部依赖

use std::collections::HashMap;
use std::sync::LazyLock;

/// 原子散射因子参数
#[derive(Debug, Clone, Copy)]
pub struct ScatteringFactorParams {
    pub a: [f64; 4],
    pub b: [f64; 4],
    pub c: f64,
}

impl ScatteringFactorParams {
    /// 计算散射因子 f(s)，其中 s = sin(θ)/λ
    pub fn calculate(&self, s: f64) -> f64 {
        let s2 = s * s;
        let mut f = self.c;
        for i in 0..4 {
            f += self.a[i] * (-self.b[i] * s2).exp();
        }
        f
    }
}

/// 原子散射因子数据库
/// 数据来自 International Tables for Crystallography
pub static SCATTERING_FACTORS: LazyLock<HashMap<&'static str, ScatteringFactorParams>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();

        // 氢 (H)
        m.insert(
            "H",
            ScatteringFactorParams {
                a: [0.493002, 0.322912, 0.140191, 0.040810],
                b: [10.5109, 26.1257, 3.14236, 57.7997],
                c: 0.003038,
            },
        );

        // 氦 (He)
        m.insert(
            "He",
            ScatteringFactorParams {
                a: [0.8734, 0.6309, 0.3112, 0.1780],
                b: [9.1037, 3.3568, 22.9276, 0.9821],
                c: 0.0064,
            },
        );

        // 锂 (Li)
        m.insert(
            "Li",
            ScatteringFactorParams {
                a: [1.1282, 0.7508, 0.6175, 0.4653],
                b: [3.9546, 1.0524, 85.3905, 168.261],
                c: 0.0377,
            },
        );

        // 铍 (Be)
        m.insert(
            "Be",
            ScatteringFactorParams {
                a: [1.5919, 1.1278, 0.5391, 0.7029],
                b: [43.6427, 1.8623, 103.483, 0.5420],
                c: 0.0385,
            },
        );

        // 硼 (B)
        m.insert(
            "B",
            ScatteringFactorParams {
                a: [2.0545, 1.3326, 1.0979, 0.7068],
                b: [23.2185, 1.0210, 60.3498, 0.1403],
                c: -0.1932,
            },
        );

        // 碳 (C)
        m.insert(
            "C",
            ScatteringFactorParams {
                a: [2.3100, 1.0200, 1.5886, 0.8650],
                b: [20.8439, 10.2075, 0.5687, 51.6512],
                c: 0.2156,
            },
        );

        // 氮 (N)
        m.insert(
            "N",
            ScatteringFactorParams {
                a: [12.2126, 3.1322, 2.0125, 1.1663],
                b: [0.0057, 9.8933, 28.9975, 0.5826],
                c: -11.529,
            },
        );

        // 氧 (O)
        m.insert(
            "O",
            ScatteringFactorParams {
                a: [3.0485, 2.2868, 1.5463, 0.8670],
                b: [13.2771, 5.7011, 0.3239, 32.9089],
                c: 0.2508,
            },
        );

        // 氟 (F)
        m.insert(
            "F",
            ScatteringFactorParams {
                a: [3.5392, 2.6412, 1.5170, 1.0243],
                b: [10.2825, 4.2944, 0.2615, 26.1476],
                c: 0.2776,
            },
        );

        // 钠 (Na)
        m.insert(
            "Na",
            ScatteringFactorParams {
                a: [4.7626, 3.1736, 1.2674, 1.1128],
                b: [3.2850, 8.8422, 0.3136, 129.424],
                c: 0.6760,
            },
        );

        // 镁 (Mg)
        m.insert(
            "Mg",
            ScatteringFactorParams {
                a: [5.4204, 2.1735, 1.2269, 2.3073],
                b: [2.8275, 79.2611, 0.3808, 7.1937],
                c: 0.8584,
            },
        );

        // 铝 (Al)
        m.insert(
            "Al",
            ScatteringFactorParams {
                a: [6.4202, 1.9002, 1.5936, 1.9646],
                b: [3.0387, 0.7426, 31.5472, 85.0886],
                c: 1.1151,
            },
        );

        // 硅 (Si)
        m.insert(
            "Si",
            ScatteringFactorParams {
                a: [6.2915, 3.0353, 1.9891, 1.5410],
                b: [2.4386, 32.3337, 0.6785, 81.6937],
                c: 1.1407,
            },
        );

        // 磷 (P)
        m.insert(
            "P",
            ScatteringFactorParams {
                a: [6.4345, 4.1791, 1.7800, 1.4908],
                b: [1.9067, 27.1570, 0.5260, 68.1645],
                c: 1.1149,
            },
        );

        // 硫 (S)
        m.insert(
            "S",
            ScatteringFactorParams {
                a: [6.9053, 5.2034, 1.4379, 1.5863],
                b: [1.4679, 22.2151, 0.2536, 56.1720],
                c: 0.8669,
            },
        );

        // 氯 (Cl)
        m.insert(
            "Cl",
            ScatteringFactorParams {
                a: [11.4604, 7.1964, 6.2556, 1.6455],
                b: [0.0104, 1.1662, 18.5194, 47.7784],
                c: -9.5574,
            },
        );

        // 钾 (K)
        m.insert(
            "K",
            ScatteringFactorParams {
                a: [8.2186, 7.4398, 1.0519, 0.8659],
                b: [12.7949, 0.7748, 213.187, 41.6841],
                c: 1.4228,
            },
        );

        // 钙 (Ca)
        m.insert(
            "Ca",
            ScatteringFactorParams {
                a: [8.6266, 7.3873, 1.5899, 1.0211],
                b: [10.4421, 0.6599, 85.7484, 178.437],
                c: 1.3751,
            },
        );

        // 钛 (Ti)
        m.insert(
            "Ti",
            ScatteringFactorParams {
                a: [9.7595, 7.3558, 1.6991, 1.9021],
                b: [7.8508, 0.5000, 35.6338, 116.105],
                c: 1.2807,
            },
        );

        // 钒 (V)
        m.insert(
            "V",
            ScatteringFactorParams {
                a: [10.2971, 7.3511, 2.0703, 2.0571],
                b: [6.8657, 0.4385, 26.8938, 102.478],
                c: 1.2199,
            },
        );

        // 铬 (Cr)
        m.insert(
            "Cr",
            ScatteringFactorParams {
                a: [10.6406, 7.3537, 3.3240, 1.4922],
                b: [6.1038, 0.3920, 20.2626, 98.7399],
                c: 1.1832,
            },
        );

        // 锰 (Mn)
        m.insert(
            "Mn",
            ScatteringFactorParams {
                a: [11.2819, 7.3573, 3.0193, 2.2441],
                b: [5.3409, 0.3432, 17.8674, 83.7543],
                c: 1.0896,
            },
        );

        // 铁 (Fe)
        m.insert(
            "Fe",
            ScatteringFactorParams {
                a: [11.7695, 7.3573, 3.5222, 2.3045],
                b: [4.7611, 0.3072, 15.3535, 76.8805],
                c: 1.0369,
            },
        );

        // 钴 (Co)
        m.insert(
            "Co",
            ScatteringFactorParams {
                a: [12.2841, 7.3409, 4.0034, 2.3488],
                b: [4.2791, 0.2784, 13.5359, 71.1692],
                c: 1.0118,
            },
        );

        // 镍 (Ni)
        m.insert(
            "Ni",
            ScatteringFactorParams {
                a: [12.8376, 7.2920, 4.4438, 2.3800],
                b: [3.8785, 0.2565, 12.1763, 66.3421],
                c: 1.0341,
            },
        );

        // 铜 (Cu)
        m.insert(
            "Cu",
            ScatteringFactorParams {
                a: [13.3380, 7.1676, 5.6158, 1.6735],
                b: [3.5828, 0.2470, 11.3966, 64.8126],
                c: 1.1910,
            },
        );

        // 锌 (Zn)
        m.insert(
            "Zn",
            ScatteringFactorParams {
                a: [14.0743, 7.0318, 5.1652, 2.4100],
                b: [3.2655, 0.2333, 10.3163, 58.7097],
                c: 1.3041,
            },
        );

        // 镓 (Ga)
        m.insert(
            "Ga",
            ScatteringFactorParams {
                a: [15.2354, 6.7006, 4.3591, 2.9623],
                b: [3.0669, 0.2412, 10.7805, 61.4135],
                c: 1.7189,
            },
        );

        // 锗 (Ge)
        m.insert(
            "Ge",
            ScatteringFactorParams {
                a: [16.0816, 6.3747, 3.7068, 3.6830],
                b: [2.8509, 0.2516, 11.4468, 54.7625],
                c: 2.1313,
            },
        );

        // 砷 (As)
        m.insert(
            "As",
            ScatteringFactorParams {
                a: [16.6723, 6.0701, 3.4313, 4.2779],
                b: [2.6345, 0.2647, 12.9479, 47.7972],
                c: 2.531,
            },
        );

        // 硒 (Se)
        m.insert(
            "Se",
            ScatteringFactorParams {
                a: [17.0006, 5.8196, 3.9731, 4.3543],
                b: [2.4098, 0.2726, 15.2372, 43.8163],
                c: 2.8409,
            },
        );

        // 溴 (Br)
        m.insert(
            "Br",
            ScatteringFactorParams {
                a: [17.1789, 5.2358, 5.6377, 3.9851],
                b: [2.1723, 16.5796, 0.2609, 41.4328],
                c: 2.9557,
            },
        );

        // 铷 (Rb)
        m.insert(
            "Rb",
            ScatteringFactorParams {
                a: [17.5816, 7.6598, 5.8981, 2.7817],
                b: [1.7139, 14.7957, 0.1603, 31.2087],
                c: 2.0782,
            },
        );

        // 锶 (Sr)
        m.insert(
            "Sr",
            ScatteringFactorParams {
                a: [17.5663, 9.8184, 5.4220, 2.6694],
                b: [1.5564, 14.0988, 0.1664, 132.376],
                c: 2.5064,
            },
        );

        // 钇 (Y)
        m.insert(
            "Y",
            ScatteringFactorParams {
                a: [17.7760, 10.2946, 5.7263, 3.2656],
                b: [1.4029, 12.8006, 0.1255, 104.354],
                c: 1.9341,
            },
        );

        // 锆 (Zr)
        m.insert(
            "Zr",
            ScatteringFactorParams {
                a: [17.8765, 10.9480, 5.4173, 3.6577],
                b: [1.2761, 11.9160, 0.1176, 87.6627],
                c: 2.0690,
            },
        );

        // 铌 (Nb)
        m.insert(
            "Nb",
            ScatteringFactorParams {
                a: [17.6142, 12.0144, 4.0418, 3.5334],
                b: [1.1886, 11.7660, 0.2047, 69.7957],
                c: 3.7553,
            },
        );

        // 钼 (Mo)
        m.insert(
            "Mo",
            ScatteringFactorParams {
                a: [3.7025, 17.2356, 12.8876, 3.7429],
                b: [0.2772, 1.0958, 11.0040, 61.6584],
                c: 4.3875,
            },
        );

        // 银 (Ag)
        m.insert(
            "Ag",
            ScatteringFactorParams {
                a: [19.2808, 16.6885, 4.8045, 1.0463],
                b: [0.6446, 7.4726, 24.6605, 99.8156],
                c: 5.1790,
            },
        );

        // 钡 (Ba)
        m.insert(
            "Ba",
            ScatteringFactorParams {
                a: [20.3361, 19.2970, 10.8880, 2.6959],
                b: [3.2160, 0.2756, 20.2073, 167.202],
                c: 2.7731,
            },
        );

        // 镧 (La)
        m.insert(
            "La",
            ScatteringFactorParams {
                a: [20.5780, 19.5990, 11.3727, 3.2879],
                b: [2.9480, 0.2440, 18.7726, 133.124],
                c: 2.1461,
            },
        );

        // 铈 (Ce)
        m.insert(
            "Ce",
            ScatteringFactorParams {
                a: [21.1671, 19.7695, 11.8513, 3.3303],
                b: [2.8129, 0.2268, 17.6083, 127.113],
                c: 1.8623,
            },
        );

        // 金 (Au)
        m.insert(
            "Au",
            ScatteringFactorParams {
                a: [16.8819, 18.5913, 25.5582, 5.8600],
                b: [0.4611, 8.6216, 1.4826, 36.3956],
                c: 12.0658,
            },
        );

        // 铅 (Pb)
        m.insert(
            "Pb",
            ScatteringFactorParams {
                a: [31.0617, 13.0637, 18.4420, 5.9696],
                b: [0.6902, 2.3576, 8.6180, 47.2579],
                c: 13.4118,
            },
        );

        // 铋 (Bi)
        m.insert(
            "Bi",
            ScatteringFactorParams {
                a: [33.3689, 12.9510, 16.5877, 6.4692],
                b: [0.7040, 2.9238, 8.7937, 48.0093],
                c: 13.5782,
            },
        );

        m
    });

/// 获取元素的原子散射因子参数
pub fn get_scattering_factor(element: &str) -> Option<&'static ScatteringFactorParams> {
    // 尝试直接匹配
    if let Some(params) = SCATTERING_FACTORS.get(element) {
        return Some(params);
    }

    // 尝试只取前两个字符（处理如 "Fe1" 这样的标签）
    let symbol: String = element.chars().take(2).collect();
    if let Some(params) = SCATTERING_FACTORS.get(symbol.as_str()) {
        return Some(params);
    }

    // 尝试只取第一个字符
    let first: String = element.chars().take(1).collect();
    SCATTERING_FACTORS.get(first.as_str())
}

/// 计算原子散射因子
/// element: 元素符号
/// s: sin(θ)/λ
pub fn calculate_scattering_factor(element: &str, s: f64) -> f64 {
    if let Some(params) = get_scattering_factor(element) {
        params.calculate(s)
    } else {
        // 未知元素，返回 0
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scattering_factor_si() {
        let params = get_scattering_factor("Si").unwrap();
        // At s = 0, f(0) ≈ Z (原子序数)
        let f0 = params.calculate(0.0);
        assert!(
            (f0 - 14.0).abs() < 1.0,
            "Si f(0) should be close to 14, got {}",
            f0
        );
    }

    #[test]
    fn test_scattering_factor_fe() {
        let params = get_scattering_factor("Fe").unwrap();
        let f0 = params.calculate(0.0);
        assert!(
            (f0 - 26.0).abs() < 1.0,
            "Fe f(0) should be close to 26, got {}",
            f0
        );
    }
}
