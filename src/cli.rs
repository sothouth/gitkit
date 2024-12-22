use std::sync::LazyLock;

use clap::{Parser, ValueEnum};

#[inline(always)]
pub fn cli() -> &'static Cli {
    static CLI: LazyLock<Cli> = LazyLock::new(Cli::parse);
    &CLI
}

#[derive(Debug, Parser)]
pub struct Cli {
    pub repo: String,
    pub path: String,
    #[clap(value_enum, default_value_t = Remove::Nothing)]
    pub remove: Remove,
    #[clap(long, short)]
    pub local: Option<String>,
    #[clap(long, short)]
    pub remote: Option<String>,
    #[clap(long, short)]
    pub keep: bool,
}

#[derive(Debug, ValueEnum, Default, Clone, Copy)]
pub enum Remove {
    #[default]
    #[clap(alias = "n")]
    Nothing,
    #[clap(alias = "c")]
    Commit,
    #[clap(alias = "p")]
    Prune,
}
