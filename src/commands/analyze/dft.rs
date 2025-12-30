//! # DFT 分析子命令实现
//!
//! 分析 VASP/CASTEP 计算结果并可视化。
//!
//! ## 功能
//! - 扫描完成的 VASP/CASTEP 计算
//! - 提取焓并重新排序
//! - 生成终端表格和 CSV 输出
//! - 可选绘制比较图
//!
//! ## 依赖关系
//! - 使用 `cli/analyze.rs` 定义的参数
//! - 使用 `parsers/outcar.rs`, `parsers/castep_out.rs`
//! - 使用 `utils/output.rs`, `utils/progress.rs`

use crate::cli::analyze::{DftArgs, DftCode};
use crate::error::{QutilityError, Result};
use crate::parsers::{castep_out, outcar};
use crate::utils::{output, progress};

use std::fs;
use std::path::Path;
use tabled::{Table, Tabled};

/// 分析结果行
#[derive(Debug, Clone, Tabled)]
struct ResultRow {
    #[tabled(rename = "Rank")]
    rank: usize,
    #[tabled(rename = "Structure")]
    structure: String,
    #[tabled(rename = "Enthalpy (eV)")]
    enthalpy: String,
    #[tabled(rename = "ΔH (eV)")]
    delta_h: String,
}

/// 执行 DFT 分析
pub fn execute(args: DftArgs) -> Result<()> {
    output::print_header("Analyzing DFT Results");

    // 验证目录
    if !args.job_dir.exists() {
        return Err(QutilityError::DirectoryNotFound {
            path: args.job_dir.display().to_string(),
        });
    }

    // 扫描所有子目录
    output::print_info(&format!(
        "Scanning '{}' for {} calculations...",
        args.job_dir.display(),
        args.code
    ));

    let entries: Vec<_> = fs::read_dir(&args.job_dir)
        .map_err(|e| QutilityError::FileReadError {
            path: args.job_dir.display().to_string(),
            source: e,
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    let pb = progress::create_progress_bar(entries.len() as u64, "Parsing");

    let mut results = Vec::new();

    for entry in &entries {
        let structure_name = entry.file_name().to_string_lossy().to_string();
        let calc_dir = entry.path();

        let dft_result = match args.code {
            DftCode::Vasp => {
                let outcar_path = calc_dir.join("OUTCAR");
                if outcar_path.exists() {
                    outcar::parse_outcar(&outcar_path, &structure_name).ok()
                } else {
                    None
                }
            }
            DftCode::Castep => {
                let castep_path = calc_dir.join(format!("{}.castep", structure_name));
                if castep_path.exists() {
                    castep_out::parse_castep_output(&castep_path, &structure_name).ok()
                } else {
                    None
                }
            }
        };

        if let Some(result) = dft_result {
            if result.is_finished && result.enthalpy_ev.is_some() {
                results.push(result);
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    if results.is_empty() {
        output::print_warning("No completed DFT calculations found with valid enthalpy.");
        return Ok(());
    }

    output::print_info(&format!("Found {} completed calculations", results.len()));

    // 按焓排序
    results.sort_by(|a, b| {
        a.enthalpy_ev
            .partial_cmp(&b.enthalpy_ev)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 找到最低焓作为参考
    let min_enthalpy = results[0].enthalpy_ev.unwrap_or(0.0);

    // 生成表格数据
    let table_rows: Vec<ResultRow> = results
        .iter()
        .take(args.top_n)
        .enumerate()
        .map(|(i, r)| {
            let h = r.enthalpy_ev.unwrap_or(0.0);
            ResultRow {
                rank: i + 1,
                structure: r.structure_name.clone(),
                enthalpy: format!("{:.6}", h),
                delta_h: format!("{:.6}", h - min_enthalpy),
            }
        })
        .collect();

    // 显示表格
    output::print_header(&format!(
        "Top {} Structures by DFT Enthalpy",
        args.top_n.min(results.len())
    ));

    let table = Table::new(&table_rows);
    println!("{}", table);

    // 保存完整 CSV
    save_results_csv(&results, &args.output_csv)?;
    output::print_success(&format!(
        "Full ranking saved to '{}'",
        args.output_csv.display()
    ));

    // 生成图表（如果请求）
    if !args.no_plot {
        if let Some(ref range) = args.plot_range {
            generate_plot(&results, range, &args.output_plot, min_enthalpy)?;
            output::print_success(&format!(
                "Comparison plot saved to '{}'",
                args.output_plot.display()
            ));
        }
    }

    Ok(())
}

/// 保存结果到 CSV
fn save_results_csv(results: &[crate::models::DftResult], output_path: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output_path).map_err(|e| QutilityError::CsvError(e))?;

    wtr.write_record(&["dft_rank", "structure", "enthalpy_eV"])
        .map_err(|e| QutilityError::CsvError(e))?;

    for (i, r) in results.iter().enumerate() {
        wtr.write_record(&[
            (i + 1).to_string(),
            r.structure_name.clone(),
            r.enthalpy_ev
                .map(|h| format!("{:.10}", h))
                .unwrap_or_default(),
        ])
        .map_err(|e| QutilityError::CsvError(e))?;
    }

    wtr.flush().map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// 生成比较图
fn generate_plot(
    results: &[crate::models::DftResult],
    range: &str,
    output_path: &Path,
    _min_enthalpy: f64,
) -> Result<()> {
    use plotters::prelude::*;

    // 解析范围
    let (start, end) = parse_range(range)?;
    let start_idx = start.saturating_sub(1);
    let end_idx = end.min(results.len());

    if start_idx >= end_idx {
        return Err(QutilityError::InvalidRange(range.to_string()));
    }

    let plot_data: Vec<(usize, f64)> = results[start_idx..end_idx]
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.enthalpy_ev.map(|h| (start + i, h)))
        .collect();

    if plot_data.is_empty() {
        return Err(QutilityError::Other("No data to plot".to_string()));
    }

    let y_min = plot_data
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::INFINITY, f64::min);
    let y_max = plot_data
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_margin = (y_max - y_min).abs() * 0.1;

    let root = BitMapBackend::new(output_path, (800, 600)).into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    let mut chart = ChartBuilder::on(&root)
        .caption("DFT Enthalpy Comparison", ("sans-serif", 24))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            (start as f64 - 0.5)..(end as f64 + 0.5),
            (y_min - y_margin)..(y_max + y_margin),
        )
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    chart
        .configure_mesh()
        .x_desc("Rank")
        .y_desc("Enthalpy (eV)")
        .draw()
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    // 绘制数据点
    chart
        .draw_series(
            plot_data
                .iter()
                .map(|(x, y)| Circle::new((*x as f64, *y), 5, RED.filled())),
        )
        .map_err(|e| QutilityError::Other(e.to_string()))?
        .label("DFT Enthalpy")
        .legend(|(x, y)| Circle::new((x + 10, y), 5, RED.filled()));

    // 连线
    chart
        .draw_series(LineSeries::new(
            plot_data.iter().map(|(x, y)| (*x as f64, *y)),
            RED.stroke_width(2),
        ))
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    // 标记最低点
    if let Some((min_x, min_y)) = plot_data
        .iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    {
        chart
            .draw_series(std::iter::once(Circle::new(
                (*min_x as f64, *min_y),
                8,
                GREEN.filled(),
            )))
            .map_err(|e| QutilityError::Other(e.to_string()))?
            .label("Lowest in Range")
            .legend(|(x, y)| Circle::new((x + 10, y), 5, GREEN.filled()));
    }

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    root.present()
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    Ok(())
}

/// 解析范围字符串 (e.g., "1-10")
fn parse_range(range: &str) -> Result<(usize, usize)> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return Err(QutilityError::InvalidRange(range.to_string()));
    }

    let start: usize = parts[0]
        .parse()
        .map_err(|_| QutilityError::InvalidRange(range.to_string()))?;
    let end: usize = parts[1]
        .parse()
        .map_err(|_| QutilityError::InvalidRange(range.to_string()))?;

    if start < 1 || end < start {
        return Err(QutilityError::InvalidRange(range.to_string()));
    }

    Ok((start, end))
}
