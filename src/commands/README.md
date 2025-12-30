# commands 模块

命令执行逻辑实现层。

## 架构位置

接收 `cli/` 解析的参数，调用 `parsers/`、`models/`、`xrd/`、`batch/` 完成业务逻辑。

## 模块结构

| 文件/目录 | 功能 |
|-----------|------|
| `convert.rs` | 结构格式批量转换 |
| `collect.rs` | DFT 结果收集 |
| `submit.rs` | Slurm 作业提交 |
| `analyze/` | 分析子命令目录 |
| ├─ `dft.rs` | DFT 结果分析与对比 |
| └─ `xrd.rs` | XRD 图样计算（批量） |
