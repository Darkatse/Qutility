//! # 批量执行器
//!
//! 并行执行批量处理任务。
//!
//! ## 功能
//! - 基于 rayon 的并行迭代
//! - 进度条显示
//! - 错误收集与汇总报告
//!
//! ## 依赖关系
//! - 被 `commands/analyze/xrd.rs` 调用
//! - 使用 `utils/progress.rs` 创建进度条
//! - 使用 `rayon` 进行并行计算

use crate::utils::progress;

use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 单个文件处理结果
#[derive(Debug, Clone)]
pub enum ProcessResult {
    /// 处理成功
    Success(String),
    /// 跳过（如文件已存在）
    Skipped(String),
    /// 处理失败
    Failed(String, String), // (文件路径, 错误信息)
}

/// 批量处理结果统计
#[derive(Debug, Default)]
pub struct BatchResult {
    /// 成功数量
    pub success: usize,
    /// 跳过数量
    pub skipped: usize,
    /// 失败数量
    pub failed: usize,
    /// 失败详情
    pub failures: Vec<(String, String)>,
}

impl BatchResult {
    /// 合并处理结果
    pub fn merge(&mut self, result: ProcessResult) {
        match result {
            ProcessResult::Success(_) => self.success += 1,
            ProcessResult::Skipped(_) => self.skipped += 1,
            ProcessResult::Failed(path, err) => {
                self.failed += 1;
                self.failures.push((path, err));
            }
        }
    }

    /// 总处理数量
    pub fn total(&self) -> usize {
        self.success + self.skipped + self.failed
    }
}

/// 批量执行器
pub struct BatchRunner {
    /// 并行作业数
    jobs: usize,
}

impl BatchRunner {
    /// 创建新的批量执行器
    pub fn new(jobs: usize) -> Self {
        let jobs = if jobs == 0 { num_cpus::get() } else { jobs };
        Self { jobs }
    }

    /// 并行处理文件列表
    pub fn run<F>(&self, files: Vec<PathBuf>, processor: F) -> BatchResult
    where
        F: Fn(&PathBuf) -> ProcessResult + Sync + Send,
    {
        let total = files.len();
        let pb = progress::create_progress_bar(total as u64, "Processing");

        let success_count = AtomicUsize::new(0);
        let skipped_count = AtomicUsize::new(0);
        let failed_count = AtomicUsize::new(0);

        // 配置 rayon 线程池
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.jobs)
            .build()
            .unwrap();

        let results: Vec<ProcessResult> = pool.install(|| {
            files
                .par_iter()
                .map(|file| {
                    let result = processor(file);

                    match &result {
                        ProcessResult::Success(_) => {
                            success_count.fetch_add(1, Ordering::Relaxed);
                        }
                        ProcessResult::Skipped(_) => {
                            skipped_count.fetch_add(1, Ordering::Relaxed);
                        }
                        ProcessResult::Failed(_, _) => {
                            failed_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    pb.inc(1);
                    result
                })
                .collect()
        });

        pb.finish_and_clear();

        // 汇总结果
        let mut batch_result = BatchResult::default();
        for result in results {
            batch_result.merge(result);
        }

        batch_result
    }
}
