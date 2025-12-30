//! # XRD 分析子命令实现
//!
//! 从结构文件计算 X 射线衍射图样。
//!
//! ## 功能
//! - 支持单文件和批量目录处理
//! - 并行计算（rayon）
//! - 可选展宽（Gaussian/Lorentzian/Pseudo-Voigt）
//! - 输出高质量图像 (PNG/SVG)
//! - 导出数据文件 (CSV/XY)
//!
//! ## 依赖关系
//! - 使用 `cli/analyze.rs` 定义的 XrdArgs
//! - 使用 `batch/` 模块进行批量处理
//! - 使用 `xrd/` 模块进行计算
//! - 使用 `parsers/` 读取结构

use crate::batch::{BatchRunner, FileCollector, ProcessResult};
use crate::cli::analyze::{parse_wavelength, BroadeningType, XrdArgs, XrdOutputFormat};
use crate::error::{QutilityError, Result};
use crate::parsers;
use crate::utils::output;
use crate::xrd::{self, XrdCalculator};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 执行 XRD 分析
pub fn execute(args: XrdArgs) -> Result<()> {
    output::print_header("X-Ray Diffraction Pattern Calculation");

    // 检测输入类型
    if args.input.is_file() {
        execute_single_file(&args)
    } else if args.input.is_dir() {
        execute_batch(&args)
    } else {
        Err(QutilityError::FileNotFound {
            path: args.input.display().to_string(),
        })
    }
}

/// 单文件模式
fn execute_single_file(args: &XrdArgs) -> Result<()> {
    output::print_info(&format!("Single file mode: '{}'", args.input.display()));

    let result = process_single_structure(&args.input, &args.output, args);

    match result {
        ProcessResult::Success(msg) => {
            output::print_success(&msg);
            Ok(())
        }
        ProcessResult::Skipped(msg) => {
            output::print_warning(&msg);
            Ok(())
        }
        ProcessResult::Failed(_, err) => Err(QutilityError::Other(err)),
    }
}

/// 批量处理模式
fn execute_batch(args: &XrdArgs) -> Result<()> {
    output::print_info(&format!("Batch mode: directory '{}'", args.input.display()));

    // 收集文件
    let collector = FileCollector::new(args.input.clone())
        .with_pattern(&args.pattern)
        .recursive(args.recursive);

    let files = collector.collect();

    if files.is_empty() {
        output::print_warning(&format!(
            "No matching files found with pattern '{}'",
            args.pattern
        ));
        return Ok(());
    }

    output::print_info(&format!("Found {} structure files", files.len()));

    // 确保输出目录存在
    fs::create_dir_all(&args.output).map_err(|e| QutilityError::FileWriteError {
        path: args.output.display().to_string(),
        source: e,
    })?;

    // 解析波长（提前解析一次，避免重复）
    let wavelength = parse_wavelength(&args.wavelength).map_err(|e| QutilityError::Other(e))?;

    output::print_info(&format!("Using wavelength: {:.4} Å", wavelength));

    // 推断输出格式
    let format = args.format.unwrap_or(XrdOutputFormat::Png);
    output::print_info(&format!("Output format: {:?}", format));

    // 创建共享配置
    let config = Arc::new(BatchXrdConfig {
        output_dir: args.output.clone(),
        wavelength,
        range: args.range.clone(),
        threshold: args.threshold,
        broadening: args.broadening,
        fwhm: args.fwhm,
        step: args.step,
        label_peaks: args.label_peaks,
        label_count: args.label_count,
        width: args.width,
        height: args.height,
        format,
        overwrite: args.overwrite,
    });

    // 并行处理
    let runner = BatchRunner::new(args.jobs);
    let result = runner.run(files, |file| process_batch_file(file, &config));

    // 打印统计
    output::print_separator();
    output::print_success(&format!(
        "Batch complete: {} success, {} skipped, {} failed",
        result.success, result.skipped, result.failed
    ));

    if !result.failures.is_empty() {
        output::print_warning("Failed files:");
        for (path, err) in result.failures.iter().take(10) {
            output::print_error(&format!("  {}: {}", path, err));
        }
        if result.failures.len() > 10 {
            output::print_warning(&format!("  ... and {} more", result.failures.len() - 10));
        }
    }

    Ok(())
}

/// 批量处理配置
struct BatchXrdConfig {
    output_dir: PathBuf,
    wavelength: f64,
    range: String,
    threshold: f64,
    broadening: BroadeningType,
    fwhm: f64,
    step: f64,
    label_peaks: bool,
    label_count: usize,
    width: u32,
    height: u32,
    format: XrdOutputFormat,
    overwrite: bool,
}

/// 处理批量模式中的单个文件
fn process_batch_file(input: &PathBuf, config: &Arc<BatchXrdConfig>) -> ProcessResult {
    // 构造输出文件名
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let ext = match config.format {
        XrdOutputFormat::Png => "png",
        XrdOutputFormat::Svg => "svg",
        XrdOutputFormat::Csv => "csv",
        XrdOutputFormat::Xy => "xy",
    };

    let output_file = config.output_dir.join(format!("{}_xrd.{}", stem, ext));

    // 检查是否已存在
    if output_file.exists() && !config.overwrite {
        return ProcessResult::Skipped(format!(
            "Output exists, skipping: {}",
            output_file.display()
        ));
    }

    // 创建临时 args 来复用单文件处理逻辑
    match process_single_structure_with_config(input, &output_file, config) {
        Ok(_) => {
            ProcessResult::Success(format!("{} -> {}", input.display(), output_file.display()))
        }
        Err(e) => ProcessResult::Failed(input.display().to_string(), e.to_string()),
    }
}

/// 使用完整配置处理单个结构
fn process_single_structure_with_config(
    input: &Path,
    output: &Path,
    config: &BatchXrdConfig,
) -> Result<()> {
    // 读取结构
    let crystal = parsers::parse_structure_file(input)?;

    // 解析范围
    let (theta_min, theta_max) = parse_range(&config.range)?;

    // 计算 XRD
    let calculator = XrdCalculator::new(config.wavelength);
    let pattern = calculator.calculate(&crystal, theta_min, theta_max)?;

    // 应用展宽
    let broadened_data = if config.broadening != BroadeningType::None {
        Some(apply_broadening(
            &pattern.peaks,
            theta_min,
            theta_max,
            config.step,
            config.fwhm,
            config.broadening,
        ))
    } else {
        None
    };

    // 输出
    match config.format {
        XrdOutputFormat::Png | XrdOutputFormat::Svg => {
            let title = crystal.name.clone();
            if let Some(ref data) = broadened_data {
                xrd::plot::generate_broadened_xrd_plot(
                    data,
                    &pattern.peaks,
                    output,
                    &title,
                    config.wavelength,
                    config.width,
                    config.height,
                    config.label_peaks,
                    config.label_count,
                    config.format == XrdOutputFormat::Svg,
                )?;
            } else {
                xrd::plot::generate_xrd_plot(
                    &pattern,
                    output,
                    &title,
                    config.width,
                    config.height,
                    config.label_peaks,
                    config.label_count,
                    config.format == XrdOutputFormat::Svg,
                )?;
            }
        }
        XrdOutputFormat::Csv => {
            if let Some(ref data) = broadened_data {
                xrd::export::broadened_to_csv(data, output)?;
            } else {
                xrd::export::to_csv(&pattern, output)?;
            }
        }
        XrdOutputFormat::Xy => {
            if let Some(ref data) = broadened_data {
                xrd::export::broadened_to_xy(
                    data,
                    &pattern.structure_name,
                    config.wavelength,
                    output,
                )?;
            } else {
                xrd::export::to_xy(&pattern, output)?;
            }
        }
    }

    Ok(())
}

/// 处理单个结构文件（完整参数版本）
fn process_single_structure(input: &Path, output: &Path, args: &XrdArgs) -> ProcessResult {
    // 读取结构
    let crystal = match parsers::parse_structure_file(input) {
        Ok(c) => c,
        Err(e) => return ProcessResult::Failed(input.display().to_string(), e.to_string()),
    };

    output::print_success(&format!(
        "Loaded structure: {} ({} atoms)",
        crystal.name,
        crystal.atoms.len()
    ));

    // 解析波长
    let wavelength = match parse_wavelength(&args.wavelength) {
        Ok(w) => w,
        Err(e) => return ProcessResult::Failed(input.display().to_string(), e),
    };
    output::print_info(&format!("Using wavelength: {:.4} Å", wavelength));

    // 解析范围
    let (theta_min, theta_max) = match parse_range(&args.range) {
        Ok(r) => r,
        Err(e) => return ProcessResult::Failed(input.display().to_string(), e.to_string()),
    };
    output::print_info(&format!("2θ range: {:.1}° - {:.1}°", theta_min, theta_max));

    // 计算 XRD
    let calculator = XrdCalculator::new(wavelength);
    let pattern = match calculator.calculate(&crystal, theta_min, theta_max) {
        Ok(p) => p,
        Err(e) => return ProcessResult::Failed(input.display().to_string(), e.to_string()),
    };

    output::print_success(&format!(
        "Calculated {} diffraction peaks",
        pattern.peaks.len()
    ));

    // 应用展宽
    let broadened_data = if args.broadening != BroadeningType::None {
        output::print_info(&format!(
            "Applying {} broadening (FWHM = {:.3}°)",
            args.broadening, args.fwhm
        ));
        Some(apply_broadening(
            &pattern.peaks,
            theta_min,
            theta_max,
            args.step,
            args.fwhm,
            args.broadening,
        ))
    } else {
        None
    };

    // 确定输出格式
    let format = args
        .format
        .unwrap_or_else(|| guess_format_from_extension(output));

    // 输出
    let result = match format {
        XrdOutputFormat::Png | XrdOutputFormat::Svg => {
            let title = args.title.clone().unwrap_or_else(|| crystal.name.clone());
            if let Some(ref data) = broadened_data {
                xrd::plot::generate_broadened_xrd_plot(
                    data,
                    &pattern.peaks,
                    output,
                    &title,
                    wavelength,
                    args.width,
                    args.height,
                    args.label_peaks,
                    args.label_count,
                    format == XrdOutputFormat::Svg,
                )
            } else {
                xrd::plot::generate_xrd_plot(
                    &pattern,
                    output,
                    &title,
                    args.width,
                    args.height,
                    args.label_peaks,
                    args.label_count,
                    format == XrdOutputFormat::Svg,
                )
            }
        }
        XrdOutputFormat::Csv => {
            if let Some(ref data) = broadened_data {
                xrd::export::broadened_to_csv(data, output)
            } else {
                xrd::export::to_csv(&pattern, output)
            }
        }
        XrdOutputFormat::Xy => {
            if let Some(ref data) = broadened_data {
                xrd::export::broadened_to_xy(data, &pattern.structure_name, wavelength, output)
            } else {
                xrd::export::to_xy(&pattern, output)
            }
        }
    };

    match result {
        Ok(_) => {
            // 显示主要峰位
            print_peak_table(&pattern.peaks, 10);
            ProcessResult::Success(format!("XRD saved to '{}'", output.display()))
        }
        Err(e) => ProcessResult::Failed(input.display().to_string(), e.to_string()),
    }
}

/// 应用峰展宽
fn apply_broadening(
    peaks: &[xrd::Peak],
    theta_min: f64,
    theta_max: f64,
    step: f64,
    fwhm: f64,
    broadening_type: BroadeningType,
) -> Vec<(f64, f64)> {
    let n_points = ((theta_max - theta_min) / step).ceil() as usize + 1;
    let mut pattern: Vec<(f64, f64)> = (0..n_points)
        .map(|i| (theta_min + i as f64 * step, 0.0))
        .collect();

    let sigma = fwhm / (2.0 * (2.0_f64.ln()).sqrt() * 2.0);
    let gamma = fwhm / 2.0;

    for peak in peaks {
        if peak.intensity < 0.1 {
            continue;
        }

        for (two_theta, intensity) in pattern.iter_mut() {
            let delta = *two_theta - peak.two_theta;

            let contribution = match broadening_type {
                BroadeningType::None => 0.0,
                BroadeningType::Gaussian => {
                    peak.intensity * (-delta * delta / (2.0 * sigma * sigma)).exp()
                }
                BroadeningType::Lorentzian => {
                    peak.intensity * gamma * gamma / (delta * delta + gamma * gamma)
                }
                BroadeningType::PseudoVoigt => {
                    let gauss = (-delta * delta / (2.0 * sigma * sigma)).exp();
                    let lorentz = gamma * gamma / (delta * delta + gamma * gamma);
                    peak.intensity * 0.5 * (gauss + lorentz)
                }
            };

            *intensity += contribution;
        }
    }

    let max_intensity = pattern.iter().map(|(_, i)| *i).fold(0.0_f64, f64::max);
    if max_intensity > 0.0 {
        for (_, intensity) in pattern.iter_mut() {
            *intensity = *intensity * 100.0 / max_intensity;
        }
    }

    pattern
}

/// 从文件扩展名推断输出格式
fn guess_format_from_extension(path: &Path) -> XrdOutputFormat {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("svg") => XrdOutputFormat::Svg,
        Some("csv") => XrdOutputFormat::Csv,
        Some("xy") | Some("dat") | Some("txt") => XrdOutputFormat::Xy,
        _ => XrdOutputFormat::Png,
    }
}

/// 解析 2θ 范围
fn parse_range(range: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return Err(QutilityError::InvalidRange(range.to_string()));
    }

    let min: f64 = parts[0]
        .parse()
        .map_err(|_| QutilityError::InvalidRange(range.to_string()))?;
    let max: f64 = parts[1]
        .parse()
        .map_err(|_| QutilityError::InvalidRange(range.to_string()))?;

    if min < 0.0 || max <= min || max > 180.0 {
        return Err(QutilityError::InvalidRange(format!(
            "{} (must be 0 <= min < max <= 180)",
            range
        )));
    }

    Ok((min, max))
}

/// 打印峰位表格
fn print_peak_table(peaks: &[xrd::Peak], count: usize) {
    use tabled::{Table, Tabled};

    #[derive(Tabled)]
    struct PeakRow {
        #[tabled(rename = "2θ (°)")]
        two_theta: String,
        #[tabled(rename = "d (Å)")]
        d_spacing: String,
        #[tabled(rename = "I (%)")]
        intensity: String,
        #[tabled(rename = "(hkl)")]
        hkl: String,
    }

    let rows: Vec<PeakRow> = peaks
        .iter()
        .take(count)
        .map(|p| PeakRow {
            two_theta: format!("{:.3}", p.two_theta),
            d_spacing: format!("{:.4}", p.d_spacing),
            intensity: format!("{:.1}", p.intensity),
            hkl: format!("({} {} {})", p.h, p.k, p.l),
        })
        .collect();

    if !rows.is_empty() {
        output::print_header(&format!("Top {} XRD Peaks", rows.len()));
        let table = Table::new(&rows);
        println!("{}", table);
    }
}
