<p align="center">
  <img src="https://img.shields.io/badge/Rust-🦀-orange?style=for-the-badge" alt="Built with Rust"/>
  <img src="https://img.shields.io/badge/HPC-Ready-blue?style=for-the-badge" alt="HPC Ready"/>
  <img src="https://img.shields.io/badge/单文件-便携-green?style=for-the-badge" alt="Portable"/>
</p>

<h1 align="center">Qutility</h1>

<p align="center">
  <strong>计算凝聚态物理的瑞士军刀</strong><br/>
  <em>因为人生苦短，不想再写第 1000 遍相同的 Python 脚本了。</em>
</p>

---

## 为什么选择 Qutility？

你是否经常：
- 计算AIRSS时又要把 `.res` 文件转成 POSCAR 了？
- 从几百个 OUTCAR 里手动找那个最低能量的结构？
- 又得写 Slurm 提交脚本了？
- 算 XRD 图谱算到崩溃？

**认识一下 Qutility** — 高通量计算材料科学的好帮手！

使用 **Rust** 🦀 构建，Qutility 具有：
- **极速体验** — 因为用 Python 解析 10000 个结构文件实在太慢了
- **单一可执行文件** — 拷到任何 HPC 集群，不用 `pip install`
- **大规模并行** — 把你的 CPU 核心全用上
- **专为物理而生** — 由物理学家开发，为物理学家服务

---

## 安装

### 从源码编译（推荐）

```bash
git clone https://github.com/Darkatse/Qutility.git
cd Qutility
cargo build --release

# 把编译好的文件拷到你喜欢的地方！
cp target/release/qutility ~/.local/bin/
```

### 直接下载二进制

从 [Releases](https://github.com/Darkatse/Qutility/releases) 下载预编译的二进制文件，放到 `$PATH` 里即可。搞定！

---

## 功能一览

| 命令 | 功能 | 并行？ |
|------|------|--------|
| `convert` | 结构文件格式互转 | ✅ 是 |
| `analyze dft` | 解析 & 排名 DFT 结果 (VASP/CASTEP) | ✅ 是 |
| `analyze xrd` | 计算 X 射线衍射图谱 | ✅ 是 |
| `collect` | 收集已完成的 DFT 作业转为 `.res` | ✅ 是 |
| `submit` | 生成并提交 Slurm 批处理作业 | — |

---

## Convert：格式转换器

像专业人士一样转换你的结构文件。

```bash
# 把所有 .res 文件转成 VASP POSCAR
qutility convert -i ./structures/ -o ./poscars/ -t poscar -j 8

# 单个文件？没问题！
qutility convert -i mystructure.res -o mystructure.cell -t cell

# 递归处理嵌套目录
qutility convert -i ./project/ -o ./output/ -t cif --recursive

# 使用 Niggli 约化（需要外部 'cabal' 命令）
qutility convert -i ./raw/ -o ./reduced/ -t cell --niggli
```

**支持的格式：**
| 输入 | 输出 |
|------|------|
| `.res` (AIRSS) | `.cell` (CASTEP) |
| `.cell` | `.cif` (晶体学标准格式) |
| POSCAR/CONTCAR | `.xyz` |
| | `.xtl` (CrystalMaker) |
| | POSCAR |

---

## Analyze DFT：结果解析器

别再手动 grep OUTCAR 文件了。让 Qutility 帮你搞定！

```bash
# 解析 VASP 结果并按能量排名
qutility analyze dft --job-dir ./calculations/ --code vasp --top-n 20

# 与 EDDP 预测对比（还能画漂亮的图！）
qutility analyze dft --job-dir ./dft_jobs/ --csv-file eddp_ranking.csv --code castep

# 导出排名结果
qutility analyze dft --job-dir ./jobs/ --code vasp --output-csv final_ranking.csv
```

**输出：**
- 按焓/能量排名的结构列表
- 对比图（如果提供了 EDDP CSV）
- 包含所有解析数据的详细 CSV

---

## Analyze XRD：衍射图谱计算

从你的结构计算出版论文级别的 XRD 图谱。

```bash
# 单个结构
qutility analyze xrd mystructure.res -o xrd_pattern.png

# 批量模式：处理整个目录
qutility analyze xrd ./structures/ -o ./xrd_output/ -j 8

# 使用高斯展宽
qutility analyze xrd input.res -o output.png --broadening gaussian --fwhm 0.15

# 导出数据用于外部绘图
qutility analyze xrd input.res -o data.csv -f csv

# 使用不同波长（Mo Kα）
qutility analyze xrd input.res -o output.png -w mo-ka
```

**特性：**
- 精确的结构因子计算
- 多种输出格式：PNG、SVG、CSV、XY
- 可配置波长（Cu Kα、Mo Kα 或自定义）
- 峰展宽（Gaussian、Lorentzian、Pseudo-Voigt）
- 可选 Miller 指数标注

---

## Collect：结果收集器

把你完成的计算汇总成单个 `.res` 文件。

```bash
# 收集 VASP 结果
qutility collect ./completed_jobs/ --code vasp --output all_structures.res

# 收集 CASTEP 结果
qutility collect ./castep_jobs/ --code castep --output collected.res
```

---

## Submit：Slurm 作业提交器

再也不用手写 Slurm 脚本了！

```bash
# 生成 CASTEP 作业（演习模式）
qutility submit --csv structures.csv --struct-dir ./cells/ --range 1-50 --dry-run

# 真正提交到 Slurm
qutility submit --csv structures.csv --struct-dir ./cells/ --range 1-50 --submit

# 自定义配置的 VASP 作业
qutility submit --csv list.csv --struct-dir ./poscars/ --range 1-20 \
    --dft vasp --vasp-np 64 --time 48:00:00 --submit
```

---

## 性能

在典型工作站测试（AMD Ryzen 9950X，16 核心）：

| 任务 | 文件数 | 耗时 |
|------|--------|------|
| 转换 10,000 个 `.res` → POSCAR | 10,000 | ~3 秒 |
| 解析 1,000 个 OUTCAR 文件 | 1,000 | ~1 秒 |
| 计算 100 个 XRD 图谱 | 100 | ~1 秒 |

*用 Python 试试看吧，我们等着。* ☕

---

## 项目架构

```
qutility
├── cli/          # 命令行参数解析 (clap)
├── commands/     # 命令执行逻辑
│   └── analyze/  # DFT 和 XRD 分析子命令
├── batch/        # 并行处理基础设施
├── models/       # Crystal, Lattice, Atom 数据结构
├── parsers/      # 文件格式解析器 (.res, .cell, POSCAR, OUTCAR...)
├── xrd/          # X 射线衍射计算引擎
├── utils/        # 输出格式化、进度条、Slurm 辅助工具
└── error.rs      # 统一错误处理
```

---

## 贡献

发现 bug？有功能建议？欢迎 PR！

```bash
# 运行测试
cargo test

# 调试模式构建
cargo build
```

---

## 许可证

Apache 2.0 许可证 — 随便用，不过原子乱跑了别怪我们 XD

---

<p align="center">
  <em>用 ❤️ 和 ☕ 为计算材料科学社区打造。</em><br/>
  <strong>祝你找到理想的结构！</strong>
</p>
