//! # Slurm 脚本生成工具
//!
//! 生成 sbatch 提交脚本。
//!
//! ## 依赖关系
//! - 被 `commands/submit.rs` 使用
//! - 无外部模块依赖

use std::path::Path;

/// Slurm 作业配置
pub struct SlurmConfig {
    pub job_name: String,
    pub partition: String,
    pub constraint: String,
    pub nodes: u32,
    pub ntasks: u32,
    pub cpus_per_task: u32,
    pub mem_per_cpu: String,
    pub time_limit: String,
    pub modules: Vec<String>,
}

impl Default for SlurmConfig {
    fn default() -> Self {
        SlurmConfig {
            job_name: "job".to_string(),
            partition: "arm".to_string(),
            constraint: "neoverse_v2".to_string(),
            nodes: 1,
            ntasks: 32,
            cpus_per_task: 1,
            mem_per_cpu: "3G".to_string(),
            time_limit: "24:00:00".to_string(),
            modules: vec![],
        }
    }
}

/// 生成 sbatch 脚本内容
pub fn generate_sbatch_script(config: &SlurmConfig, workdir: &Path, exec_cmd: &str) -> String {
    let module_loads = config
        .modules
        .iter()
        .map(|m| format!("module load {}", m))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"#!/bin/bash
#SBATCH --constraint "{}"
#SBATCH --partition {}
#SBATCH --switches=1
#SBATCH --nodes={}
#SBATCH --mem-per-cpu {}
#SBATCH --time {}
#SBATCH -c {}
#SBATCH -n {}
#SBATCH -J {}
#SBATCH -o slurm_logs/%x.out
#SBATCH -e slurm_logs/%x.err

set -euo pipefail

export MODULEPATH="/home/changjiangwu_umass_edu/Modulefiles:$MODULEPATH"
module purge 2>&1
{}
echo "Loaded modules"

cd "{}"
echo "PWD=$(pwd)"
echo "Running: {}"
{}

echo "Timings:"
sacct -o JobID,Submit,Start,End,CPUTime,State -j $SLURM_JOBID
echo "Resources:"
sacct -o JobID,JobName,Partition,ReqMem,MaxRSS,MaxVMSize -j $SLURM_JOBID
"#,
        config.constraint,
        config.partition,
        config.nodes,
        config.mem_per_cpu,
        config.time_limit,
        config.cpus_per_task,
        config.ntasks,
        config.job_name,
        module_loads,
        workdir.display(),
        exec_cmd,
        exec_cmd,
    )
}

/// 插入或替换 CASTEP .cell 文件中的 EXTERNAL_PRESSURE 块
pub fn upsert_external_pressure_block(cell_text: &str, p_gpa: f64) -> String {
    use regex::Regex;

    let block = format!(
        r#"%BLOCK EXTERNAL_PRESSURE
GPa
{p_gpa} 0 0
{p_gpa} 0
{p_gpa}
%ENDBLOCK EXTERNAL_PRESSURE
"#
    );

    // 移除已存在的 EXTERNAL_PRESSURE 块
    let pattern =
        Regex::new(r"(?is)%BLOCK\s+EXTERNAL_PRESSURE.*?%ENDBLOCK\s+EXTERNAL_PRESSURE\s*").unwrap();
    let stripped = pattern.replace_all(cell_text, "");
    let stripped = stripped.trim_end();

    format!("{}\n\n{}\n", stripped, block)
}
