extern crate clap;
extern crate crossterm;
extern crate json;
extern crate rand;

mod ostree;
use ostree::OSTree;

use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
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

#[derive(Debug)]
struct ProgressTable<'a> {
    pub entries: Vec<(ProgressEntry, &'a TableEntry)>,
    tree_passed: OSTree,
    tree_failed: OSTree,
    stp: f64,
}

const UNIT: i64 = 10000;

impl<'a> ProgressTable<'a> {
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

#[derive(Clone, Copy, Debug)]
struct ProgressEntry {
    /// Variable size from 0 to UNIT
    distrust: i64,
    pass: bool,
}

fn cls() {
    use crossterm::{cursor, terminal, ExecutableCommand};
    std::io::stdout()
        .lock()
        .execute(terminal::Clear(terminal::ClearType::All))
        .unwrap()
        .execute(cursor::MoveTo(0, 1))
        .unwrap();
}

fn standby(lines: &mut std::io::Lines<std::io::StdinLock>) {
    use crossterm::{cursor, ExecutableCommand};
    std::io::stdout().lock().execute(cursor::Hide).unwrap();
    if let Some(x) = lines.next() {
        x.unwrap();
    }
    std::io::stdout().lock().execute(cursor::Show).unwrap();
    cls();
}

fn readin(lines: &mut std::io::Lines<std::io::StdinLock>) -> Option<String> {
    use crossterm::{cursor, ExecutableCommand};
    std::io::stdout()
        .lock()
        .execute(cursor::MoveRight(4))
        .unwrap();
    let r = lines.next().map(|x| x.unwrap());
    cls();
    r
}

fn simulate(mut pt: ProgressTable, flag_debug: bool) {
    use rand::prelude::*;
    use std::io::{self, BufRead};
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
    let stdin = io::stdin();
    let lines = &mut stdin.lock().lines();
    let mut rng = rand::thread_rng();
    let mut selector = || rng.gen::<f64>();
    loop {
        if flag_debug {
            eprintln!("=== ĮSIMINIMAS ===")
        }
        //standby(lines);
        let lentries = pt.select_random_entries(LEARN_SESSIONS, false, || 0_f64);
        for lentry in lentries {
            let lte: &TableEntry = lentry.1;
            println!("    {}", lte.lhs);
            println!("    {}", lte.rhs);
            standby(lines);
            pt.set(lentry.0, true);
            if flag_debug {
                eprintln!("{:#?}", pt)
            }
            loop {
                let rentries = pt.select_random_entries(1, true, &mut selector);
                if rentries.is_empty() {
                    break;
                }
                let (ridx, rte) = rentries[0];
                println!("    {}", rte.lhs);
                let uln = readin(lines).unwrap();
                if flag_debug {
                    eprintln!("{:#?}", pt);
                }
                let rpass = rte.assess(uln);
                if flag_debug {
                    eprintln!("{}", rpass);
                }
                pt.set(ridx, rpass);
                pt.step();
                if rpass {
                    break;
                }
                println!("    {}", rte.lhs);
                println!("    {}", rte.rhs);
                standby(lines);
                if flag_debug {
                    eprintln!("{:#?}", pt);
                }
                pt.set(lentry.0, true);
            }
        }
        println!("=== SAVIKONTROLĖ ===");
        standby(lines);
        let rentries = pt.select_random_entries(ASSESS_SESSIONS, true, &mut selector);
        for rentry in rentries {
            let (ridx, rte) = rentry;
            println!("    {}", rte.lhs);
            let uln = readin(lines).unwrap();
            if flag_debug {
                eprintln!("{:#?}", pt);
            }
            let rpass = rte.assess(uln);
            if flag_debug {
                eprintln!("{}", rpass);
            }
            pt.set(ridx, rpass);
            pt.step();
        }
    }
}

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The JSON-formatted input path
    #[clap()]
    inpath: std::path::PathBuf,

    #[clap(short, long)]
    debug: bool,
}

fn main() {
    let args = Args::parse();
    cls();
    let table: Vec<TableEntry> = load_table(&args.inpath);
    let progress = ProgressTable::new(&table);
    simulate(progress, args.debug);
}
