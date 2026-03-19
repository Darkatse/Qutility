//! # Command execution module
//!
//! Implements the concrete workflows behind each top-level command.
//!
//! ## Coupling
//! - Invoked by `main.rs`
//! - Uses `cli/`, `dft/`, `parsers/`, `models/`, and `utils/`

pub mod analyze;
pub mod collect;
pub mod convert;
pub mod submit;

use crate::cli::Commands;
use crate::error::Result;

pub fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Convert(args) => convert::execute(args),
        Commands::Analyze(args) => analyze::execute(args),
        Commands::Collect(args) => collect::execute(args),
        Commands::Submit(args) => submit::execute(args),
    }
}
