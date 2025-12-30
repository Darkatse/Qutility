//! # collect 命令实现
//!
//! 收集完成的 DFT 计算结果并转换为 .res 格式。
//!
//! ## 功能
//! - 扫描完成的 VASP/CASTEP 计算
//! - 提取 CONTCAR/POSCAR 或 .cell 结构
//! - 转换为 .res 格式
//! - 合并到单个文件
//!
//! ## 依赖关系
//! - 使用 `cli/collect.rs` 定义的参数
//! - 使用 `parsers/`
//! - 使用 `utils/output.rs`, `utils/progress.rs`

use crate::cli::analyze::DftCode;
use crate::cli::collect::CollectArgs;
use crate::error::{QutilityError, Result};
use crate::parsers;
use crate::parsers::res::to_res_string;
use crate::utils::{output, progress};

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;

/// 执行 collect 命令
pub fn execute(args: CollectArgs) -> Result<()> {
    output::print_header("Collecting DFT Results");

    // 验证目录
    if !args.dft_dir.exists() {
        return Err(QutilityError::DirectoryNotFound {
            path: args.dft_dir.display().to_string(),
        });
    }

    // 扫描目录
    let entries: Vec<_> = fs::read_dir(&args.dft_dir)
        .map_err(|e| QutilityError::FileReadError {
            path: args.dft_dir.display().to_string(),
            source: e,
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    output::print_info(&format!("Scanning {} directories...", entries.len()));

    let pb = progress::create_progress_bar(entries.len() as u64, "Converting to .res");

    let mut collected_res: Vec<String> = Vec::new();
    let mut success_count = 0;

    for entry in &entries {
        let structure_name = entry.file_name().to_string_lossy().to_string();
        let calc_dir = entry.path();

        // 检查计算是否完成
        let (is_finished, structure_file) = match args.code {
            DftCode::Vasp => check_vasp_completion(&calc_dir),
            DftCode::Castep => check_castep_completion(&calc_dir, &structure_name),
        };

        if is_finished {
            if let Some(struct_file) = structure_file {
                // 转换为 .res
                let res_content = if args.use_cabal {
                    convert_to_res_cabal(&struct_file, &args.code)
                } else {
                    convert_to_res_native(&struct_file, &structure_name)
                };

                match res_content {
                    Ok(content) => {
                        collected_res.push(content);
                        success_count += 1;
                    }
                    Err(e) => {
                        pb.suspend(|| {
                            output::print_warning(&format!(
                                "Failed to convert {}: {}",
                                structure_name, e
                            ));
                        });
                    }
                }
            }
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    if collected_res.is_empty() {
        output::print_warning("No completed calculations found to collect.");
        return Ok(());
    }

    // 写入输出文件
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
    output::print_info("This file can be used for 'cryan' analysis or as EDDP training data.");

    Ok(())
}

/// 检查 VASP 计算是否完成
fn check_vasp_completion(calc_dir: &Path) -> (bool, Option<std::path::PathBuf>) {
    let outcar = calc_dir.join("OUTCAR");
    if !outcar.exists() {
        return (false, None);
    }

    let mut is_finished = false;
    if let Ok(file) = File::open(&outcar) {
        let reader = BufReader::new(file);
        for line in reader.lines().filter_map(|l| l.ok()) {
            if line.contains("General timing and accounting informations for this job") {
                is_finished = true;
                break;
            }
        }
    }

    if is_finished {
        // 优先 CONTCAR，其次 POSCAR
        let contcar = calc_dir.join("CONTCAR");
        let poscar = calc_dir.join("POSCAR");

        if contcar.exists() && contcar.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return (true, Some(contcar));
        } else if poscar.exists() {
            return (true, Some(poscar));
        }
    }

    (false, None)
}

/// 检查 CASTEP 计算是否完成
fn check_castep_completion(
    calc_dir: &Path,
    structure_name: &str,
) -> (bool, Option<std::path::PathBuf>) {
    let castep_file = calc_dir.join(format!("{}.castep", structure_name));
    if !castep_file.exists() {
        return (false, None);
    }

    let mut is_finished = false;
    if let Ok(file) = File::open(&castep_file) {
        let reader = BufReader::new(file);
        // Collect last 100 lines to check for completion
        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        for line in lines.iter().rev().take(100) {
            if line.contains("Total time") {
                is_finished = true;
                break;
            }
        }
    }

    if is_finished {
        // 优先 -out.cell，其次 .cell
        let out_cell = calc_dir.join(format!("{}-out.cell", structure_name));
        let cell = calc_dir.join(format!("{}.cell", structure_name));

        if out_cell.exists() {
            return (true, Some(out_cell));
        } else if cell.exists() {
            return (true, Some(cell));
        }
    }

    (false, None)
}

/// 原生转换为 .res
fn convert_to_res_native(struct_file: &Path, structure_name: &str) -> Result<String> {
    let mut crystal = parsers::parse_structure_file(struct_file)?;
    crystal.name = structure_name.to_string();
    Ok(to_res_string(&crystal))
}

/// 使用 cabal 转换为 .res
fn convert_to_res_cabal(struct_file: &Path, code: &DftCode) -> Result<String> {
    let input_content =
        fs::read_to_string(struct_file).map_err(|e| QutilityError::FileReadError {
            path: struct_file.display().to_string(),
            source: e,
        })?;

    // VASP POSCAR/CONTCAR 在 cabal 中用 'castep' 格式
    // CASTEP .cell 用 'cell' 格式
    let input_format = match code {
        DftCode::Vasp => "castep",
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

    use std::io::Write;
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
