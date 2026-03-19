//! # DFT 后处理子命令实现
//!
//! 对已完成并可解析的 DFT 结果进行排序、导出与可选绘图。
//!
//! ## 依赖关系
//! - 使用 `cli/analyze.rs` 定义的参数
//! - 复用 `dft/` 扫描模块与 `utils/output.rs`

use crate::cli::analyze::DftPostprocessingArgs;
use crate::dft::scan_calculations;
use crate::error::{QutilityError, Result};
use crate::models::{CalculationStatus, DftCodeType, DftResult};
use crate::utils::output;

use std::path::Path;
use tabled::{Table, Tabled};

/// 后处理结果表
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

pub fn execute(args: DftPostprocessingArgs) -> Result<()> {
    output::print_header("DFT Postprocessing");

    let code: DftCodeType = args.code.into();
    let records = scan_calculations(&args.job_dir, code)?;

    let parse_error_count = records
        .iter()
        .filter(|record| record.status == CalculationStatus::ParseError)
        .count();

    let completed_without_enthalpy = records
        .iter()
        .filter(|record| {
            record.status == CalculationStatus::Completed
                && record
                    .parsed
                    .as_ref()
                    .and_then(|result| result.enthalpy_ev)
                    .is_none()
        })
        .count();

    let mut results: Vec<DftResult> = records
        .into_iter()
        .filter_map(|record| match (record.status, record.parsed) {
            (CalculationStatus::Completed, Some(result)) if result.enthalpy_ev.is_some() => {
                Some(result)
            }
            _ => None,
        })
        .collect();

    if results.is_empty() {
        output::print_warning("No completed DFT calculations found with valid enthalpy.");
        return Ok(());
    }

    results.sort_by(|a, b| {
        a.enthalpy_ev
            .partial_cmp(&b.enthalpy_ev)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    output::print_info(&format!(
        "Found {} completed calculations with valid enthalpy",
        results.len()
    ));

    if completed_without_enthalpy > 0 {
        output::print_warning(&format!(
            "{} completed calculations were skipped because enthalpy could not be extracted",
            completed_without_enthalpy
        ));
    }

    if parse_error_count > 0 {
        output::print_warning(&format!(
            "{} completed calculations were skipped because parsing failed",
            parse_error_count
        ));
    }

    let min_enthalpy = results[0].enthalpy_ev.expect("validated before sorting");
    let table_rows: Vec<ResultRow> = results
        .iter()
        .take(args.top_n)
        .enumerate()
        .map(|(i, result)| {
            let enthalpy = result.enthalpy_ev.expect("validated before sorting");
            ResultRow {
                rank: i + 1,
                structure: result.structure_name.clone(),
                enthalpy: format!("{enthalpy:.6}"),
                delta_h: format!("{:.6}", enthalpy - min_enthalpy),
            }
        })
        .collect();

    output::print_header(&format!(
        "Top {} Structures by DFT Enthalpy",
        args.top_n.min(results.len())
    ));
    println!("{}", Table::new(&table_rows));

    save_results_csv(&results, &args.output_csv)?;
    output::print_success(&format!(
        "Full ranking saved to '{}'",
        args.output_csv.display()
    ));

    if !args.no_plot {
        if let Some(ref range) = args.plot_range {
            generate_plot(&results, range, &args.output_plot)?;
            output::print_success(&format!(
                "Comparison plot saved to '{}'",
                args.output_plot.display()
            ));
        }
    }

    Ok(())
}

fn save_results_csv(results: &[DftResult], output_path: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(output_path).map_err(QutilityError::CsvError)?;

    wtr.write_record(["dft_rank", "structure", "enthalpy_eV"])
        .map_err(QutilityError::CsvError)?;

    for (i, result) in results.iter().enumerate() {
        let enthalpy = result.enthalpy_ev.expect("validated before writing");
        wtr.write_record([
            (i + 1).to_string(),
            result.structure_name.clone(),
            format!("{enthalpy:.10}"),
        ])
        .map_err(QutilityError::CsvError)?;
    }

    wtr.flush().map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

fn generate_plot(results: &[DftResult], range: &str, output_path: &Path) -> Result<()> {
    use plotters::prelude::*;

    let (start, end) = parse_range(range)?;
    let start_idx = start.saturating_sub(1);
    let end_idx = end.min(results.len());

    if start_idx >= end_idx {
        return Err(QutilityError::InvalidRange(range.to_string()));
    }

    let plot_data: Vec<(usize, f64)> = results[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(i, result)| {
            (
                start + i,
                result.enthalpy_ev.expect("validated before plotting"),
            )
        })
        .collect();

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

    chart
        .draw_series(
            plot_data
                .iter()
                .map(|(x, y)| Circle::new((*x as f64, *y), 5, RED.filled())),
        )
        .map_err(|e| QutilityError::Other(e.to_string()))?
        .label("DFT Enthalpy")
        .legend(|(x, y)| Circle::new((x + 10, y), 5, RED.filled()));

    chart
        .draw_series(LineSeries::new(
            plot_data.iter().map(|(x, y)| (*x as f64, *y)),
            RED.stroke_width(2),
        ))
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    if let Some((min_x, min_y)) = plot_data
        .iter()
        .min_by(|a, b| a.1.partial_cmp(&b.1).expect("finite enthalpy"))
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
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    root.present()
        .map_err(|e| QutilityError::Other(e.to_string()))?;

    Ok(())
}

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
