<p align="center">
  <img src="https://img.shields.io/badge/Rust-🦀-orange?style=for-the-badge" alt="Built with Rust"/>
  <img src="https://img.shields.io/badge/HPC-Ready-blue?style=for-the-badge" alt="HPC Ready"/>
  <img src="https://img.shields.io/badge/Single_Binary-Portable-green?style=for-the-badge" alt="Portable"/>
</p>

<h1 align="center">Qutility</h1>

<p align="center">
  <strong>A Swiss Army Knife for Computational Condensed Matter Physics</strong><br/>
  <em>Because life's too short to write the same Python script 1000 times.</em>
</p>

---

## Why Qutility?

Ever found yourself:
- Converting `.res` files to POSCAR... again?
- Parsing hundreds of OUTCAR files to find that *one* low-energy structure?
- Writing yet another Slurm submission script?
- Generating XRD patterns and wishing it was faster?

**Say hello to Qutility** — your new best friend in high-throughput computational materials science!

Built with **Rust** 🦀, Qutility is:
- **Blazingly fast** — because waiting for Python to parse 10,000 structures is *so* 2019
- **Single binary** — copy it to any HPC cluster, no `pip install` headaches
- **Massively parallel** — throw all your CPU cores at the problem
- **Purpose-built** — by physicists, for physicists

---

## Installation

### From Source (Recommended)

```bash
git clone https://github.com/Darkatse/Qutility.git
cd Qutility
cargo build --release

# Copy the binary anywhere you want!
cp target/release/qutility ~/.local/bin/
```

### Just the Binary

Download the pre-built binary from [Releases](https://github.com/Darkatse/Qutility/releases) and drop it in your `$PATH`. Done.

---

## Features at a Glance

| Command | What it does | Parallel? |
|---------|--------------|-----------|
| `convert` | Convert structure files between formats | ✅ Yes |
| `analyze dft` | Parse & rank DFT results (VASP/CASTEP) | ✅ Yes |
| `analyze xrd` | Calculate X-ray diffraction patterns | ✅ Yes |
| `collect` | Gather completed DFT jobs into `.res` | ✅ Yes |
| `submit` | Generate & submit Slurm batch jobs | — |

---

## Convert: Format Converter

Transform your structures between formats like a pro.

```bash
# Convert all .res files to VASP POSCAR
qutility convert -i ./structures/ -o ./poscars/ -t poscar -j 8

# Single file? No problem!
qutility convert -i mystructure.res -o mystructure.cell -t cell

# Recursively process nested directories
qutility convert -i ./project/ -o ./output/ -t cif --recursive

# Use Niggli reduction (requires external 'cabal')
qutility convert -i ./raw/ -o ./reduced/ -t cell --niggli
```

**Supported formats:**
| Input | Output |
|-------|--------|
| `.res` (AIRSS) | `.cell` (CASTEP) |
| `.cell` | `.cif` (Crystallographic) |
| POSCAR/CONTCAR | `.xyz` |
| | `.xtl` (CrystalMaker) |
| | POSCAR |

---

## Analyze DFT: Results Parser

Stop manually grep-ing through OUTCAR files. Let Qutility do the heavy lifting.

```bash
# Parse VASP results and rank by energy
qutility analyze dft --job-dir ./calculations/ --code vasp --top-n 20

# Compare with EDDP predictions (with beautiful plots!)
qutility analyze dft --job-dir ./dft_jobs/ --csv-file eddp_ranking.csv --code castep

# Export ranked results
qutility analyze dft --job-dir ./jobs/ --code vasp --output-csv final_ranking.csv
```

**Output:**
- Ranked structure list (by enthalpy/energy per atom)
- Comparison plots (if EDDP CSV provided)
- Detailed CSV with all parsed data

---

## Analyze XRD: Diffraction Patterns

Calculate publication-quality XRD patterns from your structures.

```bash
# Single structure
qutility analyze xrd mystructure.res -o xrd_pattern.png

# Batch mode: process entire directory
qutility analyze xrd ./structures/ -o ./xrd_output/ -j 8

# With Gaussian broadening
qutility analyze xrd input.res -o output.png --broadening gaussian --fwhm 0.15

# Export data for external plotting
qutility analyze xrd input.res -o data.csv -f csv

# Different wavelength (Mo Kα)
qutility analyze xrd input.res -o output.png -w mo-ka
```

**Features:**
- Accurate structure factor calculation
- Multiple output formats: PNG, SVG, CSV, XY
- Configurable wavelength (Cu Kα, Mo Kα, or custom)
- Peak broadening (Gaussian, Lorentzian, Pseudo-Voigt)
- Optional Miller indices labeling

---

## Collect: Gather DFT Results

Harvest your completed calculations into a single `.res` file.

```bash
# Collect VASP results
qutility collect ./completed_jobs/ --code vasp --output all_structures.res

# Collect CASTEP results
qutility collect ./castep_jobs/ --code castep --output collected.res
```

---

## Submit: Slurm Job Submitter

Generate and submit batch jobs without writing Slurm scripts by hand.

```bash
# Generate CASTEP jobs (dry run)
qutility submit --csv structures.csv --struct-dir ./cells/ --range 1-50 --dry-run

# Actually submit to Slurm
qutility submit --csv structures.csv --struct-dir ./cells/ --range 1-50 --submit

# VASP with custom settings
qutility submit --csv list.csv --struct-dir ./poscars/ --range 1-20 \
    --dft vasp --vasp-np 64 --time 48:00:00 --submit
```

---

## Performance

Benchmarked on a typical workstation (AMD Ryzen 9950X, 16 cores):

| Task | Files | Time |
|------|-------|------|
| Convert 10,000 `.res` → POSCAR | 10,000 | ~2 seconds |
| Parse 1,000 OUTCAR files | 1,000 | ~1 seconds |
| Calculate 100 XRD patterns | 100 | ~1 seconds |

*Try that with Python. We'll wait.* ☕

---

## Architecture

```
qutility
├── cli/          # Command-line argument parsing (clap)
├── commands/     # Command execution logic
│   └── analyze/  # DFT & XRD analysis subcommands
├── batch/        # Parallel processing infrastructure
├── models/       # Crystal, Lattice, Atom data structures
├── parsers/      # File format parsers (.res, .cell, POSCAR, OUTCAR...)
├── xrd/          # X-ray diffraction calculation engine
├── utils/        # Output formatting, progress bars, Slurm helpers
└── error.rs      # Unified error handling
```

---

## Contributing

Found a bug? Have a feature request? PRs are welcome!

```bash
# Run tests
cargo test

# Build in debug mode
cargo build
```

---

## License

Apache 2.0 License — do whatever you want, just don't blame us if your atoms misbehave.

---

<p align="center">
  <em>Made with ❤️ and ☕ for the computational materials science community.</em><br/>
  <strong>Happy structure hunting!</strong>
</p>
