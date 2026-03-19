//! # DFT 状态扫描子命令实现
//!
//! 扫描 VASP/CASTEP 作业状态，并输出可重算结构清单。
//!
//! ## 依赖关系
//! - 使用 `cli/analyze.rs` 定义的参数
//! - 复用 `dft/` 扫描模块与 `utils/output.rs`

use crate::cli::analyze::{DftStatusArgs, RetryListFormat};
use crate::dft::{retry_candidates, scan_calculations, RetryScope};
use crate::error::{QutilityError, Result};
use crate::models::{CalculationScanRecord, CalculationStatus, DftCodeType};
use crate::utils::output;

use std::fs::File;
use std::io::Write;
use std::path::Path;
use tabled::{Table, Tabled};

#[derive(Debug, Clone, Tabled)]
struct StatusRow {
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Count")]
    count: usize,
}

#[derive(Debug, Clone, Tabled)]
struct RetryRow {
    #[tabled(rename = "Structure")]
    structure: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Reason")]
    reason: String,
}

pub fn execute(args: DftStatusArgs) -> Result<()> {
    output::print_header("DFT Job Status");

    let code: DftCodeType = args.code.into();
    let records = scan_calculations(&args.job_dir, code)?;
    let retry_scope = if args.failed_only {
        RetryScope::FailedOnly
    } else {
        RetryScope::FailedAndIncomplete
    };
    let retry_records = retry_candidates(&records, retry_scope);

    output::print_info(&format!("Scanned {} job directories", records.len()));
    print_status_summary(&records);
    print_retry_candidates(&retry_records);

    if let Some(output_path) = args.output.as_ref() {
        write_retry_list(output_path, &retry_records, args.format)?;
        output::print_success(&format!("Retry list saved to '{}'", output_path.display()));
    }

    Ok(())
}

fn print_status_summary(records: &[CalculationScanRecord]) {
    let rows = vec![
        status_row(records, CalculationStatus::Completed),
        status_row(records, CalculationStatus::Failed),
        status_row(records, CalculationStatus::Incomplete),
        status_row(records, CalculationStatus::MissingOutput),
        status_row(records, CalculationStatus::ParseError),
    ];

    output::print_header("Status Summary");
    println!("{}", Table::new(rows));
}

fn status_row(records: &[CalculationScanRecord], status: CalculationStatus) -> StatusRow {
    StatusRow {
        status: status.to_string(),
        count: records
            .iter()
            .filter(|record| record.status == status)
            .count(),
    }
}

fn print_retry_candidates(records: &[&CalculationScanRecord]) {
    output::print_header("Retry Candidates");

    if records.is_empty() {
        output::print_info("No retry candidates found.");
        return;
    }

    let rows: Vec<RetryRow> = records
        .iter()
        .map(|record| RetryRow {
            structure: record.structure_name.clone(),
            status: record.status.to_string(),
            reason: record.reason.clone().unwrap_or_default(),
        })
        .collect();

    println!("{}", Table::new(rows));
}

fn write_retry_list(
    output_path: &Path,
    records: &[&CalculationScanRecord],
    format: RetryListFormat,
) -> Result<()> {
    match format {
        RetryListFormat::Text => write_retry_list_text(output_path, records),
        RetryListFormat::Csv => write_retry_list_csv(output_path, records),
    }
}

fn write_retry_list_text(output_path: &Path, records: &[&CalculationScanRecord]) -> Result<()> {
    let mut file = File::create(output_path).map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    for record in records {
        writeln!(file, "{}", record.structure_name).map_err(|e| QutilityError::FileWriteError {
            path: output_path.display().to_string(),
            source: e,
        })?;
    }

    Ok(())
}

fn write_retry_list_csv(output_path: &Path, records: &[&CalculationScanRecord]) -> Result<()> {
    let mut writer = csv::Writer::from_path(output_path).map_err(QutilityError::CsvError)?;
    writer
        .write_record(["structure"])
        .map_err(QutilityError::CsvError)?;

    for record in records {
        writer
            .write_record([record.structure_name.as_str()])
            .map_err(QutilityError::CsvError)?;
    }

    writer.flush().map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_nanos();
        std::env::temp_dir().join(format!("qutility-dft-status-{name}-{nanos}"))
    }

    fn retry_records() -> Vec<CalculationScanRecord> {
        vec![
            CalculationScanRecord::new(
                "alpha",
                PathBuf::from("alpha"),
                DftCodeType::Vasp,
                CalculationStatus::Failed,
            ),
            CalculationScanRecord::new(
                "beta",
                PathBuf::from("beta"),
                DftCodeType::Vasp,
                CalculationStatus::Incomplete,
            ),
        ]
    }

    #[test]
    fn writes_text_retry_list() {
        let root = unique_test_dir("text");
        fs::create_dir_all(&root).expect("create root");
        let output_path = root.join("retry.txt");
        let records = retry_records();
        let refs: Vec<_> = records.iter().collect();

        write_retry_list_text(&output_path, &refs).expect("write text");

        let content = fs::read_to_string(&output_path).expect("read text");
        assert_eq!(content, "alpha\nbeta\n");

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn writes_csv_retry_list() {
        let root = unique_test_dir("csv");
        fs::create_dir_all(&root).expect("create root");
        let output_path = root.join("retry.csv");
        let records = retry_records();
        let refs: Vec<_> = records.iter().collect();

        write_retry_list_csv(&output_path, &refs).expect("write csv");

        let content = fs::read_to_string(&output_path).expect("read csv");
        assert_eq!(content.replace("\r\n", "\n"), "structure\nalpha\nbeta\n");

        fs::remove_dir_all(&root).expect("cleanup");
    }
}
