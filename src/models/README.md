# models 模块

核心数据模型定义。

## 架构位置

被 `parsers/` 写入、被 `commands/` 和 `xrd/` 读取的中心数据结构。

## 模块结构

| 文件 | 功能 |
|------|------|
| `structure.rs` | Crystal, Lattice, Atom 结构体 |
| `calculation.rs` | DftResult 计算结果模型 |
