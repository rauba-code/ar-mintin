/*
 * main.rs -- Core application
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

extern crate clap;
extern crate crossterm;
extern crate ctrlc;
extern crate json;
extern crate rand;
extern crate serde;
extern crate serde_json;

mod cli;
mod ostree;
use ostree::OSTree;

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct TableEntry {
    lhs: String,
    rhs: String,
}

impl TableEntry {
    pub fn assess(&self, user_input: String) -> bool {
        user_input == self.rhs
    }
}

fn load_table(path: &Path) -> Vec<TableEntry> {
    let input: json::JsonValue = {
        let mut file = File::open(&path).unwrap();
        let mut file_data = String::new();
        file.read_to_string(&mut file_data).unwrap();
        json::parse(&file_data).unwrap()
    };
    assert!(input["version"] == 1i32);
    let data = &input["data"];
    let table: Vec<TableEntry> = data
        .members()
        .map(|x| TableEntry {
            lhs: String::from((&x[0]).as_str().unwrap()),
            rhs: String::from((&x[1]).as_str().unwrap()),
        })
        .collect();
    table
}

#[derive(Serialize, Deserialize)]
struct ProgressTableData {
    entries: Vec<(ProgressEntry, TableEntry)>,
    stp: f64,
}

impl ProgressTableData {
    pub fn new(table: &ProgressTable) -> ProgressTableData {
        ProgressTableData {
            stp: table.stp,
            entries: table
                .entries
                .iter()
                .map(|&x| (x.0, (*x.1).clone()))
                .collect(),
        }
    }
}

#[derive(Debug)]
struct ProgressTable<'a> {
    pub entries: Vec<(ProgressEntry, &'a TableEntry)>,
    tree_passed: OSTree,
    tree_failed: OSTree,
    stp: f64,
}

const UNIT: i64 = 10000;

impl<'a> ProgressTable<'a> {
    pub fn write_to_file(&'a self, path: &Path) {
        let outdata = serde_json::to_vec(&ProgressTableData::new(self)).unwrap();
        let mut f = File::create(path).unwrap();
        f.write_all(&outdata).unwrap();
    }

    fn tree_from_entries(entries: &'a [ProgressEntry], pass: bool) -> OSTree {
        let mut tree = OSTree::new(entries.len());
        for (idx, &entry) in entries.iter().enumerate() {
            if entry.pass == pass {
                tree.assign(idx, entry.distrust);
            }
        }
        tree
    }

    pub fn new_from_file(entries: &'a [TableEntry], path: &Path) -> ProgressTable<'a> {
        use std::collections::HashMap;
        let mut buf = Vec::<u8>::new();
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        let data: ProgressTableData = serde_json::from_slice(&buf).unwrap();
        let mut imap = HashMap::new();
        for entry in data.entries {
            imap.insert(entry.1, entry.0);
        }
        let pe = || {
            entries.iter().map(|x| {
                if imap.contains_key(x) {
                    imap[x]
                } else {
                    ProgressEntry {
                        distrust: UNIT,
                        pass: false,
                    }
                }
            })
        };
        let pev: Vec<ProgressEntry> = pe().collect();
        ProgressTable {
            entries: pe().zip(entries.iter()).collect(),
            tree_passed: ProgressTable::tree_from_entries(&pev, true),
            tree_failed: ProgressTable::tree_from_entries(&pev, false),
            stp: data.stp,
        }
    }

    pub fn new(input: &'a [TableEntry]) -> ProgressTable<'a> {
        let n = input.len();
        ProgressTable {
            entries: input
                .iter()
                .map(|x| {
                    (
                        ProgressEntry {
                            distrust: UNIT,
                            pass: false,
                        },
                        x,
                    )
                })
                .collect(),
            tree_passed: OSTree::new(n),
            tree_failed: {
                let mut xt = OSTree::new(n);
                for i in 0..n {
                    xt.assign(i, UNIT);
                }
                xt
            },
            stp: UNIT as f64,
        }
    }
    pub fn select_random_entries<F>(
        &mut self,
        n: usize,
        pass: bool,
        mut selector: F,
    ) -> Vec<(usize, &'a TableEntry)>
    where
        F: FnMut() -> f64,
    {
        let tree: &mut OSTree = if pass {
            &mut self.tree_passed
        } else {
            &mut self.tree_failed
        };
        struct TreeBorrow {
            idx: usize,
            val: i64,
        }
        let mut borrows = Vec::<TreeBorrow>::new();
        for _ in 0..n {
            let sum = tree.sum();
            if sum == 0 {
                break;
            }
            let sel: i64 = (sum as f64 * selector()) as i64;
            let idx: usize = tree.rank(sel);
            borrows.push(TreeBorrow {
                idx,
                val: tree.value_at(idx),
            });
            tree.assign(idx, 0)
        }
        let result: Vec<(usize, &'a TableEntry)> = borrows
            .iter()
            .map(|x: &TreeBorrow| (x.idx, self.entries[x.idx].1))
            .collect();
        for i in borrows {
            tree.assign(i.idx, i.val);
        }
        result
    }

    pub fn set(&mut self, idx: usize, pass: bool) {
        let entry = &mut self.entries[idx];
        entry.0.pass = pass;
        entry.0.distrust = if pass {
            entry.0.distrust / 2
        } else {
            let stp = self.stp as i64;
            (entry.0.distrust + stp) / 2
        };
        self.tree_passed
            .assign(idx, if pass { entry.0.distrust } else { 0 });
        self.tree_failed
            .assign(idx, if !pass { entry.0.distrust } else { 0 });
    }

    pub fn step(&mut self) {
        let n = self.entries.len() as f64;
        let smult: f64 = 0.8_f64.powf(n.recip());
        self.stp *= smult;
        const MINPREC: i64 = 100;
        if self.stp < MINPREC as f64 {
            const MULT: i64 = UNIT / MINPREC;
            self.tree_passed.multiply(MULT);
            self.tree_failed.multiply(MULT);
            self.stp *= smult;
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ProgressEntry {
    /// Variable size from 0 to UNIT
    distrust: i64,
    pass: bool,
}

struct Simulation<'a> {
    pt: ProgressTable<'a>,
    args: Args,
}

impl<'a> Simulation<'a> {
    fn ptset(&mut self, idx: usize, val: bool) {
        self.pt.set(idx, val);
        if let Some(op) = self
            .args
            .progress
            .as_ref()
            .or(self.args.outprogress.as_ref())
        {
            self.pt.write_to_file(op)
        }
    }

    pub fn simulate(&mut self) {
        use rand::prelude::*;
        const LEARN_SESSIONS: usize = 10;
        const ASSESS_SESSIONS: usize = 10;
        /* loop {
         *   A = select X entries from non-passed entries
         *   for each I in A {
         *     learn I
         *     pick random entry
         *   }
         *   B =
         * }
         */
        let stdin = std::io::stdin();
        let lines = &mut stdin.lock().lines();
        let mut rng = rand::thread_rng();
        let mut selector = || rng.gen::<f64>();
        loop {
            if self.args.debug {
                eprintln!("=== ĮSIMINIMAS ===")
            }
            //cli::standby(lines);
            let lentries = self
                .pt
                .select_random_entries(LEARN_SESSIONS, false, || 0_f64);
            for lentry in lentries {
                let lte: &TableEntry = lentry.1;
                println!("    {}", lte.lhs);
                println!("    {}", lte.rhs);
                cli::standby(lines);
                self.ptset(lentry.0, true);
                if self.args.debug {
                    eprintln!("{:#?}", self.pt)
                }
                loop {
                    let rentries = self.pt.select_random_entries(1, true, &mut selector);
                    if rentries.is_empty() {
                        break;
                    }
                    let (ridx, rte) = rentries[0];
                    println!("    {}", rte.lhs);
                    let uln = cli::readin(lines).unwrap();
                    if self.args.debug {
                        eprintln!("{:#?}", self.pt);
                    }
                    let rpass = rte.assess(uln);
                    if self.args.debug {
                        eprintln!("{}", rpass);
                    }
                    self.ptset(ridx, rpass);
                    self.pt.step();
                    if rpass {
                        break;
                    }
                    println!("    {}", rte.lhs);
                    println!("    {}", rte.rhs);
                    cli::standby(lines);
                    if self.args.debug {
                        eprintln!("{:#?}", self.pt);
                    }
                    self.ptset(lentry.0, true);
                }
            }
            println!("=== SAVIKONTROLĖ ===");
            cli::standby(lines);
            let rentries = self
                .pt
                .select_random_entries(ASSESS_SESSIONS, true, &mut selector);
            for rentry in rentries {
                let (ridx, rte) = rentry;
                println!("    {}", rte.lhs);
                let uln = cli::readin(lines).unwrap();
                if self.args.debug {
                    eprintln!("{:#?}", self.pt);
                }
                let rpass = rte.assess(uln);
                if self.args.debug {
                    eprintln!("{}", rpass);
                }
                self.ptset(ridx, rpass);
                self.pt.step();
            }
        }
    }
}
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The path to an existing JSON-formatted input file.
    #[clap()]
    inpath: PathBuf,

    /// Path to the progress file.
    /// If the specified file does not exist,
    ///   a new file is attempted to be created on the path.
    /// Otherwise, the given file is read.
    /// If the flag is not specified, the progress is not tracked.
    #[clap(short, long)]
    progress: Option<PathBuf>,

    /// Output path to the progress file
    /// If the path is not specified,
    ///   the output path is read from --progress path.
    #[clap(short, long)]
    outprogress: Option<PathBuf>,

    /// Enable verbose debugging to STDERR.
    #[clap(short, long)]
    debug: bool,
}

fn init() {
    use crossterm::{cursor, ExecutableCommand};
    ctrlc::set_handler(|| {
        std::io::stdout().lock().execute(cursor::Show).unwrap();
        println!();
        println!("Viso gero!");
        std::process::exit(0);
    })
    .unwrap();

    print!(
        "    AR-MINTIN -- Įsiminimo programa / Memorising application
    Copyright (C) 2022 Arnoldas Rauba

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.

    Press ENTER to begin
"
    );
    cli::standby(&mut std::io::stdin().lock().lines());
}

fn get_file_type(path: &Path) -> Option<std::fs::FileType> {
    match std::fs::metadata(path) {
        Ok(m) => Some(m.file_type()),
        Err(_e) => None,
    }
}

fn main() {
    init();
    let args = Args::parse();
    cli::cls();
    let table: Vec<TableEntry> = load_table(&args.inpath);
    let ptable = if let Some(ppath) = args.progress.clone() {
        if match get_file_type(&ppath) {
            Some(pftype) => pftype.is_file(),
            None => false,
        } {
            ProgressTable::new_from_file(&table, &ppath)
        } else {
            ProgressTable::new(&table)
        }
    } else {
        ProgressTable::new(&table)
    };
    Simulation { pt: ptable, args }.simulate();
}
