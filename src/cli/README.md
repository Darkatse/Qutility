# cli 模块

命令行参数定义层 (clap)。

## 架构位置

接收用户输入，解析后传递给 `commands/` 模块执行。与 `main.rs` 直接交互。

## 模块结构

| 文件 | 功能 |
|------|------|
| `convert.rs` | convert 命令参数定义 |
| `analyze.rs` | analyze 命令及子命令参数 |
| `collect.rs` | collect 命令参数定义 |
| `submit.rs` | submit 命令参数定义 |
