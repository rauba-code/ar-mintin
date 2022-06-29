use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// The path to an existing JSON-formatted input file.
    #[clap()]
    pub inpath: PathBuf,

    /// Path to the progress file.
    /// If the specified file does not exist,
    ///   a new file is attempted to be created on the path.
    /// Otherwise, the given file is read.
    /// If the flag is not specified, the progress is not tracked.
    #[clap(short, long)]
    pub progress: Option<PathBuf>,

    /// Output path to the progress file
    /// If the path is not specified,
    ///   the output path is read from --progress path.
    #[clap(short, long)]
    pub outprogress: Option<PathBuf>,

    /// Simulate classic mode
    /// (no rehearsal of the learned sentence)
    #[clap(short, long)]
    pub classic: bool,
}
