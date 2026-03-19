//! # CLI module
//!
//! Defines the clap-based command tree for convert, analyze, collect, and submit.
//!
//! ## Coupling
//! - Used directly by `main.rs`
//! - Hands parsed arguments to `commands/`

pub mod analyze;
pub mod collect;
pub mod convert;
pub mod submit;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "qutility")]
#[command(author = "Changjiang Wu")]
#[command(version)]
#[command(about = "A unified computational condensed matter physics toolkit", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Convert structure files between formats (.res, .cell, .cif, POSCAR)
    Convert(convert::ConvertArgs),

    /// Analyze DFT job status, DFT results, or XRD patterns
    Analyze(analyze::AnalyzeArgs),

    /// Collect completed DFT results and convert to .res format
    Collect(collect::CollectArgs),

    /// Submit batch jobs to Slurm scheduler
    Submit(submit::SubmitArgs),
}
