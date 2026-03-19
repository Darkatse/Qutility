//! # DFT 作业扫描器
//!
//! 统一扫描 VASP/CASTEP 作业目录，并产出显式状态与可选解析结果。
//!
//! ## 依赖关系
//! - 被 `dft/mod.rs` 导出给命令层复用
//! - 使用 `models/calculation.rs` 和 `parsers/`

use crate::error::{QutilityError, Result};
use crate::models::{CalculationScanRecord, CalculationStatus, DftCodeType};
use crate::parsers::{castep_out, outcar};

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryScope {
    FailedAndIncomplete,
    FailedOnly,
}

impl RetryScope {
    fn matches(self, status: CalculationStatus) -> bool {
        match self {
            RetryScope::FailedAndIncomplete => {
                matches!(
                    status,
                    CalculationStatus::Failed | CalculationStatus::Incomplete
                )
            }
            RetryScope::FailedOnly => status == CalculationStatus::Failed,
        }
    }
}

pub fn scan_calculations(root: &Path, code: DftCodeType) -> Result<Vec<CalculationScanRecord>> {
    if !root.exists() {
        return Err(QutilityError::DirectoryNotFound {
            path: root.display().to_string(),
        });
    }

    let mut entries: Vec<_> = fs::read_dir(root)
        .map_err(|e| QutilityError::FileReadError {
            path: root.display().to_string(),
            source: e,
        })?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .collect();

    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_string());

    Ok(entries
        .into_iter()
        .map(|entry| {
            let structure_name = entry.file_name().to_string_lossy().to_string();
            scan_calculation(entry.path(), structure_name, code)
        })
        .collect())
}

pub fn retry_candidates(
    records: &[CalculationScanRecord],
    scope: RetryScope,
) -> Vec<&CalculationScanRecord> {
    records
        .iter()
        .filter(|record| scope.matches(record.status))
        .collect()
}

fn scan_calculation(
    calc_dir: PathBuf,
    structure_name: String,
    code: DftCodeType,
) -> CalculationScanRecord {
    let output_file = output_file_path(&calc_dir, &structure_name, code);
    let structure_file = structure_file_path(&calc_dir, &structure_name, code);

    let Some(output_file) = output_file else {
        return CalculationScanRecord::new(
            structure_name,
            calc_dir,
            code,
            CalculationStatus::MissingOutput,
        );
    };

    let inspection = match inspect_output_file(&output_file, code) {
        Ok(inspection) => inspection,
        Err(err) => {
            let mut record = CalculationScanRecord::new(
                structure_name,
                calc_dir,
                code,
                CalculationStatus::ParseError,
            );
            record.reason = Some(err.to_string());
            record.structure_file = structure_file;
            return record;
        }
    };

    if inspection.completed {
        return build_completed_record(
            calc_dir,
            structure_name,
            code,
            structure_file,
            &output_file,
        );
    }

    if let Some(reason) = inspection.failure_reason {
        let mut record =
            CalculationScanRecord::new(structure_name, calc_dir, code, CalculationStatus::Failed);
        record.reason = Some(reason);
        record.structure_file = structure_file;
        return record;
    }

    let mut record = CalculationScanRecord::new(
        structure_name,
        calc_dir,
        code,
        CalculationStatus::Incomplete,
    );
    record.structure_file = structure_file;
    record
}

fn build_completed_record(
    calc_dir: PathBuf,
    structure_name: String,
    code: DftCodeType,
    structure_file: Option<PathBuf>,
    output_file: &Path,
) -> CalculationScanRecord {
    let mut record = CalculationScanRecord::new(
        structure_name.clone(),
        calc_dir,
        code,
        CalculationStatus::Completed,
    );
    record.structure_file = structure_file;

    let parsed = match code {
        DftCodeType::Vasp => outcar::parse_outcar(output_file, &structure_name),
        DftCodeType::Castep => castep_out::parse_castep_output(output_file, &structure_name),
    };

    match parsed {
        Ok(result) => {
            record.parsed = Some(result);
            record
        }
        Err(err) => {
            record.status = CalculationStatus::ParseError;
            record.reason = Some(err.to_string());
            record
        }
    }
}

fn output_file_path(calc_dir: &Path, structure_name: &str, code: DftCodeType) -> Option<PathBuf> {
    let path = match code {
        DftCodeType::Vasp => calc_dir.join("OUTCAR"),
        DftCodeType::Castep => calc_dir.join(format!("{structure_name}.castep")),
    };

    path.exists().then_some(path)
}

fn structure_file_path(
    calc_dir: &Path,
    structure_name: &str,
    code: DftCodeType,
) -> Option<PathBuf> {
    match code {
        DftCodeType::Vasp => {
            let contcar = calc_dir.join("CONTCAR");
            if contcar.exists() && contcar.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                Some(contcar)
            } else {
                let poscar = calc_dir.join("POSCAR");
                poscar.exists().then_some(poscar)
            }
        }
        DftCodeType::Castep => {
            let out_cell = calc_dir.join(format!("{structure_name}-out.cell"));
            if out_cell.exists() {
                Some(out_cell)
            } else {
                let cell = calc_dir.join(format!("{structure_name}.cell"));
                cell.exists().then_some(cell)
            }
        }
    }
}

fn inspect_output_file(path: &Path, code: DftCodeType) -> std::io::Result<OutputInspection> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut completed = false;
    let mut failure_reason = None;

    for line in reader.lines() {
        let line = line?;

        if matches_completion(&line, code) {
            completed = true;
        }

        if failure_reason.is_none() {
            failure_reason = detect_failure_reason(&line, code);
        }
    }

    Ok(OutputInspection {
        completed,
        failure_reason,
    })
}

fn matches_completion(line: &str, code: DftCodeType) -> bool {
    match code {
        DftCodeType::Vasp => {
            line.contains("General timing and accounting informations for this job")
        }
        DftCodeType::Castep => line.contains("Total time"),
    }
}

fn detect_failure_reason(line: &str, code: DftCodeType) -> Option<String> {
    let normalized = line.to_ascii_lowercase();
    let reason = match code {
        DftCodeType::Vasp => vasp_failure_reason(&normalized),
        DftCodeType::Castep => castep_failure_reason(&normalized),
    }?;

    Some(reason.to_string())
}

fn vasp_failure_reason(line: &str) -> Option<&'static str> {
    for (pattern, reason) in [
        (
            "zbrent: fatal error in bracketing",
            "VASP ionic relaxation failed (ZBRENT)",
        ),
        (
            "brmix: very serious problems",
            "VASP electronic minimization failed (BRMIX)",
        ),
        (
            "edddav: call to zhegv failed",
            "VASP diagonalization failed (EDDDAV)",
        ),
        (
            "dav: sub-space-matrix is not hermitian",
            "VASP subspace matrix became non-hermitian",
        ),
        (
            "error in subspace rotation pssyevx",
            "VASP subspace rotation failed",
        ),
        (
            "error fexcf:",
            "VASP exchange-correlation evaluation failed",
        ),
        ("segmentation fault", "VASP crashed with segmentation fault"),
        ("forrtl:", "VASP Fortran runtime error"),
    ] {
        if line.contains(pattern) {
            return Some(reason);
        }
    }

    None
}

fn castep_failure_reason(line: &str) -> Option<&'static str> {
    for (pattern, reason) in [
        (
            "error terminating execution",
            "CASTEP terminated with an error",
        ),
        ("aborting the calculation", "CASTEP aborted the calculation"),
        (
            "segmentation fault",
            "CASTEP crashed with segmentation fault",
        ),
        ("forrtl:", "CASTEP Fortran runtime error"),
    ] {
        if line.contains(pattern) {
            return Some(reason);
        }
    }

    None
}

struct OutputInspection {
    completed: bool,
    failure_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_nanos();
        std::env::temp_dir().join(format!("qutility-dft-{name}-{nanos}"))
    }

    #[test]
    fn scan_marks_missing_output() {
        let root = unique_test_dir("missing-output");
        let job_dir = root.join("alpha");
        fs::create_dir_all(&job_dir).expect("create job dir");

        let records = scan_calculations(&root, DftCodeType::Vasp).expect("scan");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, CalculationStatus::MissingOutput);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn scan_marks_incomplete_vasp() {
        let root = unique_test_dir("incomplete-vasp");
        let job_dir = root.join("beta");
        fs::create_dir_all(&job_dir).expect("create job dir");
        fs::write(job_dir.join("OUTCAR"), "still running\n").expect("write OUTCAR");

        let records = scan_calculations(&root, DftCodeType::Vasp).expect("scan");

        assert_eq!(records[0].status, CalculationStatus::Incomplete);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn scan_marks_failed_vasp() {
        let root = unique_test_dir("failed-vasp");
        let job_dir = root.join("gamma");
        fs::create_dir_all(&job_dir).expect("create job dir");
        fs::write(
            job_dir.join("OUTCAR"),
            "BRMIX: very serious problems\ncalculation stopped\n",
        )
        .expect("write OUTCAR");

        let records = scan_calculations(&root, DftCodeType::Vasp).expect("scan");

        assert_eq!(records[0].status, CalculationStatus::Failed);
        assert!(records[0]
            .reason
            .as_deref()
            .expect("reason")
            .contains("BRMIX"));

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn scan_marks_completed_vasp() {
        let root = unique_test_dir("completed-vasp");
        let job_dir = root.join("delta");
        fs::create_dir_all(&job_dir).expect("create job dir");
        fs::write(
            job_dir.join("OUTCAR"),
            "\
enthalpy is  TOTEN    =      -12.500000 eV
energy  without entropy=     -12.500000  energy(sigma->0) =     -12.500000
  volume of cell :      123.456789
   NIONS =       8
General timing and accounting informations for this job
",
        )
        .expect("write OUTCAR");
        fs::write(job_dir.join("CONTCAR"), "contcar\n").expect("write CONTCAR");

        let records = scan_calculations(&root, DftCodeType::Vasp).expect("scan");

        assert_eq!(records[0].status, CalculationStatus::Completed);
        assert_eq!(
            records[0]
                .parsed
                .as_ref()
                .and_then(|result| result.enthalpy_ev),
            Some(-12.5)
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn retry_scope_filters_records() {
        let failed = CalculationScanRecord::new(
            "failed",
            PathBuf::from("failed"),
            DftCodeType::Vasp,
            CalculationStatus::Failed,
        );
        let incomplete = CalculationScanRecord::new(
            "incomplete",
            PathBuf::from("incomplete"),
            DftCodeType::Vasp,
            CalculationStatus::Incomplete,
        );
        let completed = CalculationScanRecord::new(
            "completed",
            PathBuf::from("completed"),
            DftCodeType::Vasp,
            CalculationStatus::Completed,
        );

        let records = vec![failed, incomplete, completed];

        assert_eq!(
            retry_candidates(&records, RetryScope::FailedAndIncomplete).len(),
            2
        );
        assert_eq!(retry_candidates(&records, RetryScope::FailedOnly).len(), 1);
    }
}
