//! # submit 命令实现
//!
//! 批量提交 CASTEP/VASP 作业到 Slurm。
//!
//! ## 功能
//! - 读取结构列表 CSV
//! - 生成作业目录和输入文件
//! - 生成 sbatch 脚本
//! - 可选自动提交
//!
//! ## 依赖关系
//! - 使用 `cli/submit.rs` 定义的参数
//! - 使用 `utils/slurm.rs`, `utils/output.rs`

use crate::cli::submit::{DftEngine, SubmitArgs};
use crate::error::{QutilityError, Result};
use crate::utils::output;
use crate::utils::slurm::{generate_sbatch_script, upsert_external_pressure_block, SlurmConfig};

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 执行 submit 命令
pub fn execute(args: SubmitArgs) -> Result<()> {
    output::print_header("Batch Job Submission");

    // 验证 CSV
    if !args.csv.exists() {
        return Err(QutilityError::FileNotFound {
            path: args.csv.display().to_string(),
        });
    }

    // 验证结构目录
    if !args.struct_dir.exists() {
        return Err(QutilityError::DirectoryNotFound {
            path: args.struct_dir.display().to_string(),
        });
    }

    // 创建作业根目录
    fs::create_dir_all(&args.jobs_root).map_err(|e| QutilityError::FileWriteError {
        path: args.jobs_root.display().to_string(),
        source: e,
    })?;

    // 读取 CSV
    let structures = read_csv_structures(&args.csv)?;
    output::print_info(&format!("Loaded {} structures from CSV", structures.len()));

    // 解析范围
    let indices = parse_range(&args.range)?;
    output::print_info(&format!(
        "Selected {} structures from range '{}'",
        indices.len(),
        args.range
    ));

    let mut submitted = Vec::new();
    let mut generated = Vec::new();

    for idx in &indices {
        let i = *idx;
        if i < 1 || i > structures.len() {
            output::print_warning(&format!("Index {} out of range, skipping", i));
            continue;
        }

        let structure_name = &structures[i - 1];
        if structure_name.is_empty() {
            output::print_warning(&format!("Empty structure name at index {}, skipping", i));
            continue;
        }

        // 查找结构文件
        let (cell_path, poscar_path) = find_structure_files(&args.struct_dir, structure_name);

        // 决定使用哪个 DFT 代码
        let chosen_dft = match args.dft {
            DftEngine::Auto => {
                if cell_path.is_some() {
                    DftEngine::Castep
                } else if poscar_path.is_some() {
                    DftEngine::Vasp
                } else {
                    output::print_warning(&format!(
                        "No .cell or POSCAR found for '{}', skipping",
                        structure_name
                    ));
                    continue;
                }
            }
            other => other,
        };

        // 创建作业目录
        let job_dir = args.jobs_root.join(structure_name);
        fs::create_dir_all(&job_dir).map_err(|e| QutilityError::FileWriteError {
            path: job_dir.display().to_string(),
            source: e,
        })?;

        // 创建 slurm_logs 目录
        fs::create_dir_all(job_dir.join("slurm_logs")).ok();

        // 根据 DFT 代码生成输入
        let sbatch_path = match chosen_dft {
            DftEngine::Castep => {
                if let Some(cell_src) = cell_path {
                    prepare_castep_job(&args, &job_dir, structure_name, &cell_src)?
                } else {
                    output::print_warning(&format!("No .cell file for CASTEP: {}", structure_name));
                    continue;
                }
            }
            DftEngine::Vasp => {
                if let Some(poscar_src) = poscar_path {
                    prepare_vasp_job(&args, &job_dir, structure_name, &poscar_src)?
                } else {
                    output::print_warning(&format!("No POSCAR for VASP: {}", structure_name));
                    continue;
                }
            }
            DftEngine::Auto => unreachable!(),
        };

        generated.push(structure_name.clone());

        // 提交作业
        if args.submit && !args.dry_run {
            match Command::new("sbatch")
                .arg(&sbatch_path)
                .current_dir(&job_dir)
                .output()
            {
                Ok(out) if out.status.success() => {
                    output::print_success(&format!(
                        "Submitted: {} - {}",
                        structure_name,
                        String::from_utf8_lossy(&out.stdout).trim()
                    ));
                    submitted.push(structure_name.clone());
                }
                Ok(out) => {
                    output::print_error(&format!(
                        "sbatch failed for {}: {}",
                        structure_name,
                        String::from_utf8_lossy(&out.stderr)
                    ));
                }
                Err(e) => {
                    output::print_error(&format!(
                        "Failed to run sbatch for {}: {}",
                        structure_name, e
                    ));
                }
            }
        } else {
            output::print_info(&format!("[DRY] Generated job: {}", job_dir.display()));
        }
    }

    output::print_separator();
    output::print_done(&format!(
        "Processed {} entries, generated {} jobs, submitted {} jobs",
        indices.len(),
        generated.len(),
        submitted.len()
    ));

    Ok(())
}

/// 读取 CSV 中的结构名称列表
fn read_csv_structures(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path).map_err(|e| QutilityError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;

    let reader = BufReader::new(file);
    let mut structures = Vec::new();
    let mut first_line = true;

    for line in reader.lines() {
        let line = line.map_err(|e| QutilityError::FileReadError {
            path: path.display().to_string(),
            source: e,
        })?;

        // 跳过空行
        if line.trim().is_empty() {
            continue;
        }

        // 第一行可能是 header
        if first_line {
            first_line = false;
            // 检查是否是 header（包含 'structure' 字样）
            if line.to_lowercase().contains("structure") {
                continue;
            }
        }

        // 取第一列作为结构名
        let name = line.split_whitespace().next().unwrap_or("").to_string();
        structures.push(name);
    }

    Ok(structures)
}

/// 解析范围字符串 (e.g., "1-5,8,10-12")
fn parse_range(expr: &str) -> Result<Vec<usize>> {
    let mut items = Vec::new();

    for chunk in expr.split(',') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }

        if chunk.contains('-') {
            let parts: Vec<&str> = chunk.splitn(2, '-').collect();
            if parts.len() != 2 {
                return Err(QutilityError::InvalidRange(chunk.to_string()));
            }
            let a: usize = parts[0]
                .parse()
                .map_err(|_| QutilityError::InvalidRange(chunk.to_string()))?;
            let b: usize = parts[1]
                .parse()
                .map_err(|_| QutilityError::InvalidRange(chunk.to_string()))?;
            if a < 1 || b < a {
                return Err(QutilityError::InvalidRange(chunk.to_string()));
            }
            items.extend(a..=b);
        } else {
            let v: usize = chunk
                .parse()
                .map_err(|_| QutilityError::InvalidRange(chunk.to_string()))?;
            if v < 1 {
                return Err(QutilityError::InvalidRange(chunk.to_string()));
            }
            items.push(v);
        }
    }

    items.sort();
    items.dedup();
    Ok(items)
}

/// 查找结构文件
fn find_structure_files(
    struct_dir: &Path,
    structure_name: &str,
) -> (Option<PathBuf>, Option<PathBuf>) {
    let cell = struct_dir.join(format!("{}.cell", structure_name));
    let cell = if cell.exists() { Some(cell) } else { None };

    // POSCAR 可能有多种位置
    let poscar_candidates = [
        struct_dir.join(format!("{}.POSCAR", structure_name)),
        struct_dir.join(structure_name).join("POSCAR"),
        struct_dir.join(format!("POSCAR_{}", structure_name)),
    ];

    let poscar = poscar_candidates.into_iter().find(|p| p.exists());

    (cell, poscar)
}

/// 准备 CASTEP 作业
fn prepare_castep_job(
    args: &SubmitArgs,
    job_dir: &Path,
    structure_name: &str,
    cell_src: &Path,
) -> Result<PathBuf> {
    // 检查 param 模板
    let param_template = args.param_template.as_ref().ok_or_else(|| {
        QutilityError::InvalidArgument("CASTEP requires --param-template".to_string())
    })?;

    let seed = structure_name;
    let dest_cell = job_dir.join(format!("{}.cell", seed));
    let dest_param = job_dir.join(format!("{}.param", seed));

    // 复制并修改 .cell 文件
    let mut cell_content =
        fs::read_to_string(cell_src).map_err(|e| QutilityError::FileReadError {
            path: cell_src.display().to_string(),
            source: e,
        })?;

    // 添加外部压力（如果指定）
    if let Some(p_gpa) = args.external_pressure {
        cell_content = upsert_external_pressure_block(&cell_content, p_gpa);
    }

    fs::write(&dest_cell, cell_content).map_err(|e| QutilityError::FileWriteError {
        path: dest_cell.display().to_string(),
        source: e,
    })?;

    // 复制 .param 模板
    fs::copy(param_template, &dest_param).map_err(|e| QutilityError::FileWriteError {
        path: dest_param.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
    })?;

    // 生成 sbatch 脚本
    let modules: Vec<String> = args
        .castep_modules
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let config = SlurmConfig {
        job_name: seed.to_string(),
        partition: args.partition.clone(),
        constraint: args.constraint.clone(),
        nodes: args.nodes,
        ntasks: args.ntasks,
        cpus_per_task: args.cpus_per_task,
        mem_per_cpu: args.mem_per_cpu.clone(),
        time_limit: args.time.clone(),
        modules,
    };

    let exec_cmd = format!(
        "mpirun -np {} {} \"{}\"",
        args.castep_np, args.castep_exec, seed
    );

    let sbatch_content = generate_sbatch_script(&config, job_dir, &exec_cmd);
    let sbatch_path = job_dir.join("submit.sbatch");

    fs::write(&sbatch_path, sbatch_content).map_err(|e| QutilityError::FileWriteError {
        path: sbatch_path.display().to_string(),
        source: e,
    })?;

    Ok(sbatch_path)
}

/// 准备 VASP 作业
fn prepare_vasp_job(
    args: &SubmitArgs,
    job_dir: &Path,
    structure_name: &str,
    poscar_src: &Path,
) -> Result<PathBuf> {
    // 检查必需的模板
    let incar_template = args.incar_template.as_ref().ok_or_else(|| {
        QutilityError::InvalidArgument("VASP requires --incar-template".to_string())
    })?;
    let kpoints_template = args.kpoints_template.as_ref().ok_or_else(|| {
        QutilityError::InvalidArgument("VASP requires --kpoints-template".to_string())
    })?;

    // 复制文件
    fs::copy(poscar_src, job_dir.join("POSCAR")).map_err(|e| QutilityError::FileWriteError {
        path: job_dir.join("POSCAR").display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
    })?;

    fs::copy(incar_template, job_dir.join("INCAR")).map_err(|e| QutilityError::FileWriteError {
        path: job_dir.join("INCAR").display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
    })?;

    fs::copy(kpoints_template, job_dir.join("KPOINTS")).map_err(|e| {
        QutilityError::FileWriteError {
            path: job_dir.join("KPOINTS").display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        }
    })?;

    // POTCAR 处理
    let potcar_dst = job_dir.join("POTCAR");
    if !potcar_dst.exists() {
        if let Some(ref potcar_dir) = args.potcar_dir {
            let potcar_src = potcar_dir.join("POTCAR");
            if potcar_src.exists() {
                fs::copy(&potcar_src, &potcar_dst).map_err(|e| QutilityError::FileWriteError {
                    path: potcar_dst.display().to_string(),
                    source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
                })?;
            } else {
                output::print_warning(&format!(
                    "POTCAR not found in {}, job may fail",
                    potcar_dir.display()
                ));
            }
        } else {
            output::print_warning(&format!(
                "No POTCAR for {}, please provide manually",
                structure_name
            ));
        }
    }

    // 生成 sbatch 脚本
    let modules: Vec<String> = args
        .vasp_modules
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let config = SlurmConfig {
        job_name: structure_name.to_string(),
        partition: args.partition.clone(),
        constraint: args.constraint.clone(),
        nodes: args.nodes,
        ntasks: args.ntasks,
        cpus_per_task: args.cpus_per_task,
        mem_per_cpu: args.mem_per_cpu.clone(),
        time_limit: args.time.clone(),
        modules,
    };

    let exec_cmd = format!("mpirun -np {} {}", args.vasp_np, args.vasp_exec);
    let sbatch_content = generate_sbatch_script(&config, job_dir, &exec_cmd);
    let sbatch_path = job_dir.join("submit.sbatch");

    fs::write(&sbatch_path, sbatch_content).map_err(|e| QutilityError::FileWriteError {
        path: sbatch_path.display().to_string(),
        source: e,
    })?;

    Ok(sbatch_path)
}
