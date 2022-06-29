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
mod ent;
mod ostree;

use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::path::Path;

use ent::TableEntry;

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

mod args;
use ent::ProgressTable;
struct Simulation<'a> {
    pt: ProgressTable<'a>,
    args: args::Args,
}

impl<'a> Simulation<'a> {
    fn ptset(&mut self, idx: usize, val: bool) {
        self.pt.set(idx, val);
        if let Some(op) = self
            .args
            .outprogress
            .as_ref()
            .or(self.args.progress.as_ref())
        {
            self.pt.write_to_file(op)
        }
    }

    fn show_entry(
        &mut self,
        ent: (usize, &TableEntry),
        lines: &mut std::io::Lines<std::io::StdinLock>,
    ) {
        println!("    {}", ent.1.lhs);
        println!("    {}", ent.1.rhs);
        cli::standby(lines);
        self.ptset(ent.0, true);
    }

    fn assess_entry(
        &mut self,
        ent: (usize, &TableEntry),
        lines: &mut std::io::Lines<std::io::StdinLock>,
    ) -> bool {
        println!("    {}", ent.1.lhs);
        let uln = cli::readin(lines).unwrap();
        let rpass = ent.1.assess(uln);
        self.ptset(ent.0, rpass);
        self.pt.step();
        rpass
    }

    pub fn simulate(&mut self) {
        use rand::prelude::*;
        const LEARN_SESSIONS: usize = 10;
        const ASSESS_SESSIONS: usize = 10;
        let stdin = std::io::stdin();
        let lines = &mut stdin.lock().lines();
        let mut rng = rand::thread_rng();
        let mut selector = || rng.gen::<f64>();
        loop {
            let lentries = self
                .pt
                .select_random_entries(LEARN_SESSIONS, false, || 0_f64);
            for lentry in lentries {
                self.show_entry(lentry, lines);
                loop {
                    let rentries = self.pt.select_random_entries(1, true, &mut selector);
                    if rentries.is_empty() {
                        break;
                    }
                    if self.assess_entry(rentries[0], lines) {
                        break;
                    }
                    self.show_entry(lentry, lines)
                }
            }
            println!("=== SAVIKONTROLĖ ===");
            cli::standby(lines);
            let rentries = self
                .pt
                .select_random_entries(ASSESS_SESSIONS, true, &mut selector);
            for rentry in rentries {
                self.assess_entry(rentry, lines);
            }
        }
    }
}
use clap::Parser;
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
    let args = args::Args::parse();
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
