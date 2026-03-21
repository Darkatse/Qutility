//! # convert 命令实现
//!
//! 批量转换结构文件格式。
//!
//! ## 功能
//! - 读取 `.res/.cell/POSCAR/CONTCAR` 等结构文件（按扩展名或文件名推断）
//! - 转换为 `.res/.cell/.cif/.xyz/.xtl/POSCAR` 格式
//! - 支持并行处理
//! - 可选使用外部 `cabal` 命令作为 fallback
//!
//! ## 依赖关系
//! - 使用 `cli/convert.rs` 定义的参数
//! - 使用 `parsers/`, `models/`
//! - 使用 `utils/output.rs`, `utils/progress.rs`

use crate::cli::convert::{ConvertArgs, OutputFormat};
use crate::error::{QutilityError, Result};
use crate::parsers;
use crate::parsers::cell::to_cell_string;
use crate::parsers::poscar::to_poscar_string;
use crate::parsers::res::to_res_string;
use crate::utils::{output, progress};

use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;

/// 执行 convert 命令
pub fn execute(args: ConvertArgs) -> Result<()> {
    output::print_header(&format!("Converting to {} format", args.target));

    // 验证输入目录
    if !args.input.exists() {
        return Err(QutilityError::DirectoryNotFound {
            path: args.input.display().to_string(),
        });
    }

    // 创建输出目录
    fs::create_dir_all(&args.output).map_err(|e| QutilityError::FileWriteError {
        path: args.output.display().to_string(),
        source: e,
    })?;

    // 收集输入文件
    let files = collect_input_files(&args.input, &args.pattern, args.recursive)?;

    if files.is_empty() {
        output::print_warning(&format!(
            "No files matched '{}' under {}",
            args.pattern,
            args.input.display()
        ));
        return Ok(());
    }

    output::print_info(&format!("Found {} files to convert", files.len()));

    // 如果需要 Niggli 归约但未使用 cabal，给出警告
    if args.niggli && !args.use_cabal {
        output::print_warning("Niggli reduction requires --use-cabal flag. Ignoring --niggli.");
    }

    // 设置并行度
    let num_threads = if args.jobs == 0 {
        num_cpus::get()
    } else {
        args.jobs
    };

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .ok();

    let pb = progress::create_progress_bar(files.len() as u64, "Converting");
    let success_count = AtomicUsize::new(0);
    let skip_count = AtomicUsize::new(0);

    // 并行处理
    files.par_iter().for_each(|input_path| {
        let result = if args.use_cabal {
            convert_with_cabal(
                input_path,
                &args.output,
                args.target,
                args.niggli,
                args.overwrite,
            )
        } else {
            convert_native(input_path, &args.output, args.target, args.overwrite)
        };

        match result {
            Ok(ConvertStatus::Success) => {
                success_count.fetch_add(1, Ordering::SeqCst);
            }
            Ok(ConvertStatus::Skipped) => {
                skip_count.fetch_add(1, Ordering::SeqCst);
            }
            Err(e) => {
                pb.suspend(|| {
                    output::print_error(&format!("{}: {}", input_path.display(), e));
                });
            }
        }
        pb.inc(1);
    });

    pb.finish_with_message("Done");

    output::print_done(&format!(
        "Converted {} file(s) to '{}' in '{}' ({} skipped)",
        success_count.load(Ordering::SeqCst),
        args.target,
        args.output.display(),
        skip_count.load(Ordering::SeqCst)
    ));

    Ok(())
}

enum ConvertStatus {
    Success,
    Skipped,
}

/// 收集输入文件
fn collect_input_files(input_dir: &Path, pattern: &str, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    let walker = if recursive {
        WalkDir::new(input_dir)
    } else {
        WalkDir::new(input_dir).max_depth(1)
    };

    let glob_pattern = glob::Pattern::new(pattern).map_err(|e| {
        QutilityError::InvalidArgument(format!("Invalid pattern '{}': {}", pattern, e))
    })?;

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if glob_pattern.matches(name) {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

/// 原生 Rust 转换
fn convert_native(
    input_path: &Path,
    output_dir: &Path,
    target: OutputFormat,
    overwrite: bool,
) -> Result<ConvertStatus> {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("structure");

    let output_path = match target {
        OutputFormat::Res => output_dir.join(format!("{}.res", stem)),
        OutputFormat::Cell => output_dir.join(format!("{}.cell", stem)),
        OutputFormat::Cif => output_dir.join(format!("{}.cif", stem)),
        OutputFormat::Xyz => output_dir.join(format!("{}.xyz", stem)),
        OutputFormat::Xtl => output_dir.join(format!("{}.xtl", stem)),
        OutputFormat::Poscar => output_dir.join(format!("POSCAR_{}", stem)),
    };

    // 检查是否需要跳过
    if output_path.exists() && !overwrite {
        return Ok(ConvertStatus::Skipped);
    }

    // 解析输入文件
    let crystal = parsers::parse_structure_file(input_path)?;

    // 转换为目标格式
    let content = match target {
        OutputFormat::Res => to_res_string(&crystal),
        OutputFormat::Cell => to_cell_string(&crystal),
        OutputFormat::Poscar => to_poscar_string(&crystal),
        OutputFormat::Cif => to_cif_string(&crystal),
        OutputFormat::Xyz => to_xyz_string(&crystal),
        OutputFormat::Xtl => to_xtl_string(&crystal),
    };

    // 写入文件
    fs::write(&output_path, content).map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    Ok(ConvertStatus::Success)
}

fn infer_cabal_format(input_path: &Path) -> Result<&'static str> {
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "res" => Ok("res"),
        "shx" => Ok("shx"),
        "cell" => Ok("cell"),
        "cif" => Ok("cif"),
        "xyz" => Ok("xyz"),
        "xtl" => Ok("xtl"),
        "vasp" | "poscar" | "contcar" => Ok("poscar"),
        _ => {
            if let Some(name) = input_path.file_name().and_then(|n| n.to_str()) {
                let upper = name.to_uppercase();
                if upper.starts_with("POSCAR") || upper.starts_with("CONTCAR") {
                    return Ok("poscar");
                }
            }

            Err(QutilityError::UnsupportedFormat(format!(
                "Cannot determine cabal format for: {}",
                input_path.display()
            )))
        }
    }
}

/// 使用外部 cabal 命令转换（fallback 模式）
fn convert_with_cabal(
    input_path: &Path,
    output_dir: &Path,
    target: OutputFormat,
    niggli: bool,
    overwrite: bool,
) -> Result<ConvertStatus> {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("structure");

    let cabal_source = infer_cabal_format(input_path)?;

    let (output_path, cabal_target) = match target {
        OutputFormat::Res => (output_dir.join(format!("{}.res", stem)), "res"),
        OutputFormat::Cell => (output_dir.join(format!("{}.cell", stem)), "cell"),
        OutputFormat::Cif => (output_dir.join(format!("{}.cif", stem)), "cif"),
        OutputFormat::Xyz => (output_dir.join(format!("{}.xyz", stem)), "xyz"),
        OutputFormat::Xtl => (output_dir.join(format!("{}.xtl", stem)), "xtl"),
        OutputFormat::Poscar => (output_dir.join(format!("POSCAR_{}", stem)), "poscar"),
    };

    if output_path.exists() && !overwrite {
        return Ok(ConvertStatus::Skipped);
    }

    // 读取输入文件
    let input_content =
        fs::read_to_string(input_path).map_err(|e| QutilityError::FileReadError {
            path: input_path.display().to_string(),
            source: e,
        })?;

    let output_content = if niggli {
        // source -> cell -> cell (niggli) -> target
        let cell1 = if cabal_source == "cell" {
            input_content.clone()
        } else {
            run_cabal(cabal_source, "cell", &input_content)?
        };
        let cell2 = run_cabal("cell", "cell", &cell1)?; // Niggli reduction
        if cabal_target == "cell" {
            cell2
        } else {
            run_cabal("cell", cabal_target, &cell2)?
        }
    } else {
        if cabal_source == cabal_target {
            input_content
        } else {
            run_cabal(cabal_source, cabal_target, &input_content)?
        }
    };

    fs::write(&output_path, output_content).map_err(|e| QutilityError::FileWriteError {
        path: output_path.display().to_string(),
        source: e,
    })?;

    Ok(ConvertStatus::Success)
}

/// 调用 cabal 命令
fn run_cabal(from: &str, to: &str, input: &str) -> Result<String> {
    let mut child = Command::new("cabal")
        .args([from, to])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| QutilityError::CommandNotFound {
            command: "cabal".to_string(),
        })?;

    use std::io::Write;
    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(input.as_bytes()).ok();
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
            command: format!("cabal {} {}", from, to),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

// ─────────────────────────────────────────────────────────────
// 原生格式转换函数
// ─────────────────────────────────────────────────────────────

/// 转换为 CIF 格式
fn to_cif_string(crystal: &crate::models::Crystal) -> String {
    let (a, b, c, alpha, beta, gamma) = crystal.lattice.parameters();

    let mut result = String::new();
    result.push_str(&format!("data_{}\n", crystal.name.replace(' ', "_")));
    result.push_str("_symmetry_space_group_name_H-M    'P 1'\n");
    result.push_str("_symmetry_Int_Tables_number       1\n\n");

    result.push_str(&format!("_cell_length_a    {:.6}\n", a));
    result.push_str(&format!("_cell_length_b    {:.6}\n", b));
    result.push_str(&format!("_cell_length_c    {:.6}\n", c));
    result.push_str(&format!("_cell_angle_alpha {:.4}\n", alpha));
    result.push_str(&format!("_cell_angle_beta  {:.4}\n", beta));
    result.push_str(&format!("_cell_angle_gamma {:.4}\n\n", gamma));

    result.push_str("loop_\n");
    result.push_str("_atom_site_label\n");
    result.push_str("_atom_site_type_symbol\n");
    result.push_str("_atom_site_fract_x\n");
    result.push_str("_atom_site_fract_y\n");
    result.push_str("_atom_site_fract_z\n");
    result.push_str("_atom_site_occupancy\n");

    for (i, atom) in crystal.atoms.iter().enumerate() {
        let label = atom
            .label
            .clone()
            .unwrap_or_else(|| format!("{}{}", atom.element, i + 1));
        result.push_str(&format!(
            "{} {} {:.10} {:.10} {:.10} 1.0\n",
            label, atom.element, atom.position[0], atom.position[1], atom.position[2]
        ));
    }

    result
}

/// 转换为 XYZ 格式
fn to_xyz_string(crystal: &crate::models::Crystal) -> String {
    let mut result = String::new();
    result.push_str(&format!("{}\n", crystal.atoms.len()));
    result.push_str(&format!("{}\n", crystal.name));

    let m = crystal.lattice.matrix;
    for atom in &crystal.atoms {
        // 分数坐标转笛卡尔坐标
        let x =
            atom.position[0] * m[0][0] + atom.position[1] * m[1][0] + atom.position[2] * m[2][0];
        let y =
            atom.position[0] * m[0][1] + atom.position[1] * m[1][1] + atom.position[2] * m[2][1];
        let z =
            atom.position[0] * m[0][2] + atom.position[1] * m[1][2] + atom.position[2] * m[2][2];
        result.push_str(&format!(
            "{} {:16.10} {:16.10} {:16.10}\n",
            atom.element, x, y, z
        ));
    }

    result
}

/// 转换为 XTL 格式 (CrystalMaker)
fn to_xtl_string(crystal: &crate::models::Crystal) -> String {
    let (a, b, c, alpha, beta, gamma) = crystal.lattice.parameters();

    let mut result = String::new();
    result.push_str(&format!("TITLE {}\n", crystal.name));
    result.push_str(&format!(
        "CELL\n  {:.6} {:.6} {:.6} {:.4} {:.4} {:.4}\n",
        a, b, c, alpha, beta, gamma
    ));
    result.push_str("SYMMETRY NUMBER 1\n");
    result.push_str("SYMMETRY LABEL P1\n");
    result.push_str("ATOMS\n");
    result.push_str("NAME       X          Y          Z\n");

    for atom in &crystal.atoms {
        result.push_str(&format!(
            "{:4} {:10.6} {:10.6} {:10.6}\n",
            atom.element, atom.position[0], atom.position[1], atom.position[2]
        ));
    }

    result.push_str("EOF\n");
    result
}
