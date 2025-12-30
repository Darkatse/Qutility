//! # 文件收集器
//!
//! 根据输入路径和模式收集待处理文件列表。
//!
//! ## 功能
//! - 支持单文件和目录输入
//! - glob 模式匹配
//! - 递归目录搜索
//!
//! ## 依赖关系
//! - 被 `commands/analyze/xrd.rs` 调用
//! - 使用 `walkdir` 遍历目录

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 文件收集器
pub struct FileCollector {
    /// 输入路径
    input: PathBuf,
    /// 匹配模式列表
    patterns: Vec<String>,
    /// 是否递归
    recursive: bool,
}

impl FileCollector {
    /// 创建新的文件收集器
    pub fn new(input: PathBuf) -> Self {
        Self {
            input,
            patterns: vec!["*".to_string()],
            recursive: false,
        }
    }

    /// 设置匹配模式（逗号分隔的多模式）
    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.patterns = pattern
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if self.patterns.is_empty() {
            self.patterns = vec!["*".to_string()];
        }
        self
    }

    /// 设置是否递归搜索
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// 检查输入是否为单文件
    pub fn is_single_file(&self) -> bool {
        self.input.is_file()
    }

    /// 检查输入是否为目录
    pub fn is_directory(&self) -> bool {
        self.input.is_dir()
    }

    /// 收集所有匹配的文件
    pub fn collect(&self) -> Vec<PathBuf> {
        if self.input.is_file() {
            return vec![self.input.clone()];
        }

        if !self.input.is_dir() {
            return vec![];
        }

        let max_depth = if self.recursive { usize::MAX } else { 1 };

        let walker = WalkDir::new(&self.input)
            .max_depth(max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());

        walker
            .filter(|entry| self.matches_patterns(entry.path()))
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    /// 检查文件是否匹配任一模式
    fn matches_patterns(&self, path: &Path) -> bool {
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        for pattern in &self.patterns {
            if Self::glob_match(pattern, filename) {
                return true;
            }
        }

        false
    }

    /// 简单 glob 匹配（支持 * 和 ? 通配符）
    fn glob_match(pattern: &str, text: &str) -> bool {
        let pattern = pattern.as_bytes();
        let text = text.as_bytes();

        let mut p = 0;
        let mut t = 0;
        let mut star_p = None;
        let mut star_t = 0;

        while t < text.len() {
            if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == text[t]) {
                p += 1;
                t += 1;
            } else if p < pattern.len() && pattern[p] == b'*' {
                star_p = Some(p);
                star_t = t;
                p += 1;
            } else if let Some(sp) = star_p {
                p = sp + 1;
                star_t += 1;
                t = star_t;
            } else {
                return false;
            }
        }

        while p < pattern.len() && pattern[p] == b'*' {
            p += 1;
        }

        p == pattern.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(FileCollector::glob_match("*.res", "test.res"));
        assert!(FileCollector::glob_match("*.res", "TiC-123.res"));
        assert!(!FileCollector::glob_match("*.res", "test.cell"));
        assert!(FileCollector::glob_match("POSCAR*", "POSCAR"));
        assert!(FileCollector::glob_match("POSCAR*", "POSCAR_001"));
        assert!(FileCollector::glob_match("test?.res", "test1.res"));
        assert!(!FileCollector::glob_match("test?.res", "test12.res"));
    }
}
