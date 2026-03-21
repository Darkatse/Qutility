//! # collect 命令实现
//!
//! 收集已完成的 DFT 结构，并转换为单个 `.res` 文件。
//!
//! ## 依赖关系
//! - 使用 `cli/collect.rs` 定义的参数
//! - 复用 `dft/` 扫描模块和 `parsers/`

use crate::cli::analyze::DftCode;
use crate::cli::collect::CollectArgs;
use crate::dft::scan_calculations;
use crate::error::{QutilityError, Result};
use crate::models::{CalculationStatus, DftCodeType, DftResult};
use crate::parsers;
use crate::parsers::res::to_res_string;
use crate::utils::{output, progress};

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn execute(args: CollectArgs) -> Result<()> {
    output::print_header("Collecting DFT Results");

    let code: DftCodeType = args.code.into();
    let records = scan_calculations(&args.dft_dir, code)?;
    let completed_records: Vec<_> = records
        .into_iter()
        .filter(|record| record.status == CalculationStatus::Completed)
        .collect();

    output::print_info(&format!(
        "Found {} completed calculations",
        completed_records.len()
    ));

    let pb = progress::create_progress_bar(completed_records.len() as u64, "Converting to .res");

    let mut collected_res = Vec::new();
    let mut success_count = 0;
    let mut missing_structure_count = 0;

    for record in completed_records {
        let Some(structure_file) = record.structure_file.as_deref() else {
            missing_structure_count += 1;
            pb.inc(1);
            continue;
        };

        let res_content = if args.use_cabal {
            convert_to_res_cabal(structure_file, &args.code)
        } else {
            convert_to_res_native(structure_file, &record.structure_name, record.parsed.as_ref())
        };

        match res_content {
            Ok(content) => {
                collected_res.push(content);
                success_count += 1;
            }
            Err(err) => {
                pb.suspend(|| {
                    output::print_warning(&format!(
                        "Failed to convert {}: {}",
                        record.structure_name, err
                    ));
                });
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    if collected_res.is_empty() {
        output::print_warning("No completed calculations found to collect.");
        return Ok(());
    }

    let mut outfile = File::create(&args.output).map_err(|e| QutilityError::FileWriteError {
        path: args.output.display().to_string(),
        source: e,
    })?;

    for res in &collected_res {
        outfile
            .write_all(res.as_bytes())
            .map_err(|e| QutilityError::FileWriteError {
                path: args.output.display().to_string(),
                source: e,
            })?;
        outfile
            .write_all(b"\n")
            .map_err(|e| QutilityError::FileWriteError {
                path: args.output.display().to_string(),
                source: e,
            })?;
    }

    output::print_done(&format!(
        "Collected {} structures into '{}'",
        success_count,
        args.output.display()
    ));

    if missing_structure_count > 0 {
        output::print_warning(&format!(
            "{} completed calculations were skipped because no structure file was found",
            missing_structure_count
        ));
    }

    output::print_info("This file can be used for 'cryan' analysis or as EDDP training data.");
    Ok(())
}

fn convert_to_res_native(
    struct_file: &Path,
    structure_name: &str,
    parsed: Option<&DftResult>,
) -> Result<String> {
    let mut crystal = parsers::parse_structure_file(struct_file)?;
    crystal.name = structure_name.to_string();

    if let Some(parsed) = parsed {
        crystal.enthalpy = parsed.enthalpy_ev.or(parsed.energy_ev);
        crystal.energy = parsed.energy_ev;
        crystal.volume = parsed.volume;
        crystal.pressure = parsed.pressure_kbar.map(|kbar| kbar * 0.1);

        if let Some(expected) = parsed.num_atoms {
            if expected != crystal.atoms.len() {
                return Err(QutilityError::InvalidArgument(format!(
                    "Atom count mismatch for {structure_name}: output says {expected}, structure file has {}",
                    crystal.atoms.len()
                )));
            }
        }
    }

    Ok(to_res_string(&crystal))
}

fn convert_to_res_cabal(struct_file: &Path, code: &DftCode) -> Result<String> {
    let input_content =
        fs::read_to_string(struct_file).map_err(|e| QutilityError::FileReadError {
            path: struct_file.display().to_string(),
            source: e,
        })?;

    let input_format = match code {
        DftCode::Vasp => "poscar",
        DftCode::Castep => "cell",
    };

    let mut child = Command::new("cabal")
        .args([input_format, "res"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| QutilityError::CommandNotFound {
            command: "cabal".to_string(),
        })?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(input_content.as_bytes()).ok();
    }

    let output = child
        .wait_with_output()
        .map_err(|e| QutilityError::CommandFailed {
            command: "cabal".to_string(),
            stderr: e.to_string(),
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(QutilityError::CommandFailed {
            command: format!("cabal {} res", input_format),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}
