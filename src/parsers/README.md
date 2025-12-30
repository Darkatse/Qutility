# parsers 模块

结构文件与 DFT 输出解析器。

## 架构位置

读取外部文件，转换为 `models/` 中定义的数据结构，供 `commands/` 使用。

## 模块结构

| 文件 | 功能 |
|------|------|
| `res.rs` | AIRSS .res 格式解析 |
| `cell.rs` | CASTEP .cell 格式解析 |
| `poscar.rs` | VASP POSCAR/CONTCAR 解析 |
| `outcar.rs` | VASP OUTCAR 结果解析 |
| `castep_out.rs` | CASTEP .castep 结果解析 |
