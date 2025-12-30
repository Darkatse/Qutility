# xrd 模块

X 射线衍射 (XRD) 图样计算与可视化。

## 架构位置

核心算法库，被 `commands/analyze/xrd.rs` 调用实现 XRD 分析功能。

## 模块结构

| 文件 | 功能 |
|------|------|
| `calculator.rs` | XRD 衍射计算核心算法 |
| `scattering.rs` | 原子散射因子数据库 (ITC Vol. C) |
| `plot.rs` | 图表生成 (plotters) |
| `export.rs` | 数据导出 (CSV/XY) |
