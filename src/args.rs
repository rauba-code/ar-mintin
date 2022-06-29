/*
 * args.rs -- Argument parser
 * Copyright (C) 2022 Arnoldas Rauba
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

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
