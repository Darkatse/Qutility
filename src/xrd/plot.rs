//! # XRD 图表生成
//!
//! 使用 `plotters` 库生成高质量 XRD 衍射图谱。
//!
//! ## 功能
//! - 论文级别的图表质量
//! - 支持 stick pattern 和 broadened pattern
//! - 可选峰位 hkl 标注
//! - 支持 PNG 和 SVG 输出
//!
//! ## 依赖关系
//! - 被 `commands/analyze/xrd.rs` 调用
//! - 使用 `xrd/calculator.rs` 的 Peak, XrdPattern 结构
//! - 使用 `plotters` 渲染图表

use crate::error::{QutilityError, Result};
use crate::xrd::{Peak, XrdPattern};

use plotters::prelude::*;
use std::path::Path;

/// 生成 XRD 图表 (stick pattern)
#[allow(clippy::too_many_arguments)]
pub fn generate_xrd_plot(
    pattern: &XrdPattern,
    output_path: &Path,
    title: &str,
    width: u32,
    height: u32,
    label_peaks: bool,
    label_count: usize,
    use_svg: bool,
) -> Result<()> {
    if use_svg {
        generate_svg(
            pattern,
            output_path,
            title,
            width,
            height,
            label_peaks,
            label_count,
        )
    } else {
        generate_png(
            pattern,
            output_path,
            title,
            width,
            height,
            label_peaks,
            label_count,
        )
    }
}

/// 生成展宽的 XRD 图表 (continuous pattern)
#[allow(clippy::too_many_arguments)]
pub fn generate_broadened_xrd_plot(
    data: &[(f64, f64)],
    peaks: &[Peak],
    output_path: &Path,
    title: &str,
    wavelength: f64,
    width: u32,
    height: u32,
    label_peaks: bool,
    label_count: usize,
    use_svg: bool,
) -> Result<()> {
    if use_svg {
        let root = SVGBackend::new(output_path, (width, height)).into_drawing_area();
        draw_broadened_chart(
            &root,
            data,
            peaks,
            title,
            wavelength,
            label_peaks,
            label_count,
        )?;
        root.present()
            .map_err(|e| QutilityError::Other(e.to_string()))?;
    } else {
        let root = BitMapBackend::new(output_path, (width, height)).into_drawing_area();
        draw_broadened_chart(
            &root,
            data,
            peaks,
            title,
            wavelength,
            label_peaks,
            label_count,
        )?;
        root.present()
            .map_err(|e| QutilityError::Other(e.to_string()))?;
    }
    Ok(())
}

/// 绘制展宽图表
#[allow(clippy::too_many_arguments)]
fn draw_broadened_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    data: &[(f64, f64)],
    peaks: &[Peak],
    title: &str,
    wavelength: f64,
    label_peaks: bool,
    label_count: usize,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    root.fill(&WHITE)
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    // 确定范围
    let x_min = data.first().map(|(x, _)| *x).unwrap_or(5.0);
    let x_max = data.last().map(|(x, _)| *x).unwrap_or(90.0);

    let mut chart = ChartBuilder::on(root)
        .caption(title, ("sans-serif", 28).into_font())
        .margin(30)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(x_min..x_max, 0.0..110.0)
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    chart
        .configure_mesh()
        .x_desc("2θ (°)")
        .y_desc("Relative Intensity (%)")
        .x_label_style(("sans-serif", 16))
        .y_label_style(("sans-serif", 16))
        .axis_desc_style(("sans-serif", 18))
        .draw()
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    // 绘制连续曲线
    let line_color = RGBColor(0, 102, 204);
    chart
        .draw_series(LineSeries::new(
            data.iter().map(|(x, y)| (*x, *y)),
            line_color.stroke_width(2),
        ))
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    // 填充曲线下方区域
    let fill_color = RGBColor(0, 102, 204).mix(0.2);
    chart
        .draw_series(AreaSeries::new(
            data.iter().map(|(x, y)| (*x, *y)),
            0.0,
            fill_color,
        ))
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    // 标注峰位
    if label_peaks {
        for peak in peaks.iter().take(label_count) {
            if peak.intensity < 5.0 {
                continue;
            }

            // 找到对应位置的强度
            let y_pos = data
                .iter()
                .find(|(x, _)| (*x - peak.two_theta).abs() < 0.05)
                .map(|(_, y)| *y)
                .unwrap_or(peak.intensity);

            let label = format!("({}{}{}) ", peak.h, peak.k, peak.l);
            let text_style = ("sans-serif", 11).into_font().color(&BLACK);

            chart
                .draw_series(std::iter::once(Text::new(
                    label,
                    (peak.two_theta, y_pos + 3.0),
                    text_style,
                )))
                .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;
        }
    }

    // 添加波长信息
    let wavelength_text = format!("λ = {:.4} Å", wavelength);
    chart
        .draw_series(std::iter::once(Text::new(
            wavelength_text,
            (x_max - 15.0, 105.0),
            ("sans-serif", 14).into_font().color(&BLACK),
        )))
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    Ok(())
}

/// 生成 PNG 图表
fn generate_png(
    pattern: &XrdPattern,
    output_path: &Path,
    title: &str,
    width: u32,
    height: u32,
    label_peaks: bool,
    label_count: usize,
) -> Result<()> {
    let root = BitMapBackend::new(output_path, (width, height)).into_drawing_area();
    draw_xrd_chart(&root, pattern, title, label_peaks, label_count)?;
    root.present()
        .map_err(|e| QutilityError::Other(e.to_string()))?;
    Ok(())
}

/// 生成 SVG 图表
fn generate_svg(
    pattern: &XrdPattern,
    output_path: &Path,
    title: &str,
    width: u32,
    height: u32,
    label_peaks: bool,
    label_count: usize,
) -> Result<()> {
    let root = SVGBackend::new(output_path, (width, height)).into_drawing_area();
    draw_xrd_chart(&root, pattern, title, label_peaks, label_count)?;
    root.present()
        .map_err(|e| QutilityError::Other(e.to_string()))?;
    Ok(())
}

/// 绘制 XRD 图表的核心逻辑 (stick pattern)
fn draw_xrd_chart<DB: DrawingBackend>(
    root: &DrawingArea<DB, plotters::coord::Shift>,
    pattern: &XrdPattern,
    title: &str,
    label_peaks: bool,
    label_count: usize,
) -> Result<()>
where
    DB::ErrorType: 'static,
{
    root.fill(&WHITE)
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    let (x_min, x_max) = if pattern.peaks.is_empty() {
        (5.0, 90.0)
    } else {
        let min_theta = pattern
            .peaks
            .iter()
            .map(|p| p.two_theta)
            .fold(f64::INFINITY, f64::min);
        let max_theta = pattern
            .peaks
            .iter()
            .map(|p| p.two_theta)
            .fold(f64::NEG_INFINITY, f64::max);
        (min_theta.floor() - 2.0, max_theta.ceil() + 2.0)
    };

    let mut chart = ChartBuilder::on(root)
        .caption(title, ("sans-serif", 28).into_font())
        .margin(30)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(x_min..x_max, 0.0..110.0)
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    chart
        .configure_mesh()
        .x_desc("2θ (°)")
        .y_desc("Relative Intensity (%)")
        .x_label_style(("sans-serif", 16))
        .y_label_style(("sans-serif", 16))
        .axis_desc_style(("sans-serif", 18))
        .draw()
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    let line_color = RGBColor(0, 102, 204);

    for peak in &pattern.peaks {
        if peak.intensity < 0.5 {
            continue;
        }

        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![(peak.two_theta, 0.0), (peak.two_theta, peak.intensity)],
                line_color.stroke_width(2),
            )))
            .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;
    }

    if label_peaks {
        let top_peaks: Vec<_> = pattern.peaks.iter().take(label_count).collect();

        for peak in top_peaks {
            if peak.intensity < 5.0 {
                continue;
            }

            let label = format!("({}{}{}) ", peak.h, peak.k, peak.l);
            let text_style = ("sans-serif", 12).into_font().color(&BLACK);

            chart
                .draw_series(std::iter::once(Text::new(
                    label,
                    (peak.two_theta, peak.intensity + 3.0),
                    text_style,
                )))
                .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;
        }
    }

    let wavelength_text = format!("λ = {:.4} Å", pattern.wavelength);
    chart
        .draw_series(std::iter::once(Text::new(
            wavelength_text,
            (x_max - 15.0, 105.0),
            ("sans-serif", 14).into_font().color(&BLACK),
        )))
        .map_err(|e| QutilityError::Other(format!("{:?}", e)))?;

    Ok(())
}
