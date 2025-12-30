//! # XRD 数据导出
//!
//! 导出 XRD 数据到 CSV 和 XY 格式。
//!
//! ## 支持格式
//! - CSV: 包含 2θ, d, intensity, hkl 的完整数据（峰位），或 2θ, intensity（展宽）
//! - XY: 标准 XRD 数据交换格式（2θ, intensity）
//!
//! ## 依赖关系
//! - 被 `commands/analyze/xrd.rs` 调用
//! - 使用 `xrd/calculator.rs` 的 XrdPattern 结构
//! - 使用 `csv` 库写入 CSV 文件

use crate::error::{QutilityError, Result};
use crate::xrd::XrdPattern;

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// 导出峰位为 CSV 格式
pub fn to_csv(pattern: &XrdPattern, output_path: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output_path).map_err(|e| QutilityError::CsvError(e))?;

    wtr.write_record(&["2theta", "d_spacing", "intensity", "h", "k", "l"])
        .map_err(|e| QutilityError::CsvError(e))?;

    let mut peaks = pattern.peaks.clone();
    peaks.sort_by(|a, b| a.two_theta.partial_cmp(&b.two_theta).unwrap());

    for peak in &peaks {
        wtr.write_record(&[
            format!("{:.4}", peak.two_theta),
            format!("{:.6}", peak.d_spacing),
            format!("{:.2}", peak.intensity),
            peak.h.to_string(),
            peak.k.to_string(),
            peak.l.to_string(),
        ])
        .map_err(|e| QutilityError::CsvError(e))?;
    }

    wtr.flush().map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// 导出展宽数据为 CSV 格式
pub fn broadened_to_csv(data: &[(f64, f64)], output_path: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output_path).map_err(|e| QutilityError::CsvError(e))?;

    wtr.write_record(&["2theta", "intensity"])
        .map_err(|e| QutilityError::CsvError(e))?;

    for (two_theta, intensity) in data {
        wtr.write_record(&[format!("{:.4}", two_theta), format!("{:.4}", intensity)])
            .map_err(|e| QutilityError::CsvError(e))?;
    }

    wtr.flush().map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// 导出峰位为 XY 格式
pub fn to_xy(pattern: &XrdPattern, output_path: &Path) -> Result<()> {
    let mut file = File::create(output_path).map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    writeln!(file, "# XRD Pattern: {}", pattern.structure_name).map_err(|e| {
        QutilityError::FileWriteError {
            path: output_path.display().to_string(),
            source: e,
        }
    })?;
    writeln!(file, "# Wavelength: {:.6} Angstrom", pattern.wavelength).map_err(|e| {
        QutilityError::FileWriteError {
            path: output_path.display().to_string(),
            source: e,
        }
    })?;
    writeln!(file, "# Columns: 2theta (degrees), Intensity (relative)").map_err(|e| {
        QutilityError::FileWriteError {
            path: output_path.display().to_string(),
            source: e,
        }
    })?;
    writeln!(file, "#").map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    let mut peaks = pattern.peaks.clone();
    peaks.sort_by(|a, b| a.two_theta.partial_cmp(&b.two_theta).unwrap());

    for peak in &peaks {
        writeln!(file, "{:.4}\t{:.2}", peak.two_theta, peak.intensity).map_err(|e| {
            QutilityError::FileWriteError {
                path: output_path.display().to_string(),
                source: e,
            }
        })?;
    }

    Ok(())
}

/// 导出展宽数据为 XY 格式
pub fn broadened_to_xy(
    data: &[(f64, f64)],
    structure_name: &str,
    wavelength: f64,
    output_path: &Path,
) -> Result<()> {
    let mut file = File::create(output_path).map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    writeln!(file, "# XRD Pattern: {} (broadened)", structure_name).map_err(|e| {
        QutilityError::FileWriteError {
            path: output_path.display().to_string(),
            source: e,
        }
    })?;
    writeln!(file, "# Wavelength: {:.6} Angstrom", wavelength).map_err(|e| {
        QutilityError::FileWriteError {
            path: output_path.display().to_string(),
            source: e,
        }
    })?;
    writeln!(file, "# Columns: 2theta (degrees), Intensity (relative)").map_err(|e| {
        QutilityError::FileWriteError {
            path: output_path.display().to_string(),
            source: e,
        }
    })?;
    writeln!(file, "#").map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    for (two_theta, intensity) in data {
        writeln!(file, "{:.4}\t{:.4}", two_theta, intensity).map_err(|e| {
            QutilityError::FileWriteError {
                path: output_path.display().to_string(),
                source: e,
            }
        })?;
    }

    Ok(())
}
