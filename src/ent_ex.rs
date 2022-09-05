/*
 * ent.rs -- Data structures for entry processing
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

use crate::ostree::OSTree;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableEntry {
    pub lhs: String,
    pub rhs: String,
}

impl TableEntry {
    pub fn assess(&self, user_input: String) -> bool {
        user_input == self.rhs
    }
}

#[derive(Serialize, Deserialize)]
struct ProgressTableData {
    entries: Vec<(ProgressEntry, TableEntry)>,
    stp: f64,
}

impl ProgressTableData {
    pub fn new(table: &ProgressTable, te: &[TableEntry]) -> ProgressTableData {
        ProgressTableData {
            stp: table.stp,
            entries: table
                .entries
                .iter()
                .zip(te)
                .map(|(&x, y)| (x, y.clone()))
                .collect(),
        }
    }
}

pub type Idx = usize;

#[derive(Debug)]
pub struct ProgressTable {
    pub entries: Vec<ProgressEntry>,
    capacity: usize,
    cnt_failed: usize,
    tree_passed: OSTree,
    tree_failed: OSTree,
    stp: f64,
}

pub const UNIT: i64 = 10000;

pub struct OutOfRangeError;

impl ProgressTable {
    pub fn write_to_file(&self, te: &[TableEntry], path: &Path) {
        let outdata = serde_json::to_vec(&ProgressTableData::new(self, te)).unwrap();
        let mut f = File::create(path).unwrap();
        f.write_all(&outdata).unwrap();
    }

    fn tree_from_entries(entries: &[ProgressEntry], pass: bool) -> OSTree {
        let mut tree = OSTree::new(entries.len());
        for (idx, &entry) in entries.iter().enumerate() {
            if entry.pass == pass {
                tree.assign(idx, entry.distrust);
            }
        }
        tree
    }

    pub fn is_partial(&self) -> bool {
        self.entries.len() < self.capacity
    }

    pub fn get_unpassed_entries_count(&self) -> usize {
        self.cnt_failed
    }

    pub fn get_unit(&self) -> f64 {
        self.stp
    }

    pub fn new_from_file(entries: &[TableEntry], path: &Path) -> ProgressTable {
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
                        distrust: data.stp as i64,
                        pass: false,
                    }
                }
            })
        };
        let pev: Vec<ProgressEntry> = pe().collect();
        ProgressTable {
            entries: pev.clone(),
            capacity: entries.len(),
            cnt_failed: pev.iter().filter(|x: &&ProgressEntry| !x.pass).count(),
            tree_passed: ProgressTable::tree_from_entries(&pev, true),
            tree_failed: ProgressTable::tree_from_entries(&pev, false),
            stp: data.stp,
        }
    }

    pub fn new(entries: Pin<Arc<Vec<TableEntry>>>) -> ProgressTable {
        let n = entries.len();
        Self::new_partial(entries, n, UNIT as f64)
    }

    pub fn new_empty(capacity: usize, stp: f64) -> ProgressTable {
        ProgressTable {
            entries: Vec::new(),
            capacity,
            cnt_failed: 0,
            tree_passed: OSTree::new(capacity),
            tree_failed: OSTree::new(capacity),
            stp,
        }
    }

    pub fn new_partial(
        entries: Pin<Arc<Vec<TableEntry>>>,
        capacity: usize,
        stp: f64,
    ) -> ProgressTable {
        let n = entries.len();
        ProgressTable {
            entries: vec![
                ProgressEntry {
                    distrust: stp as i64,
                    pass: false,
                };
                n
            ],
            capacity,
            cnt_failed: n,
            tree_passed: OSTree::new(capacity),
            tree_failed: {
                let mut xt = OSTree::new(capacity);
                for i in 0..n {
                    xt.assign(i, stp as i64);
                }
                xt
            },
            stp,
        }
    }

    pub fn supply(&mut self, m: usize) -> Result<(), OutOfRangeError> {
        let n = self.entries.len();
        if n + m > self.capacity {
            Err(OutOfRangeError)
        } else {
            self.entries.extend(vec![ProgressEntry {
                distrust: self.stp as i64,
                pass: false,
            }]);
            for i in n..(n + m) {
                self.tree_failed.assign(i, self.stp as i64);
            }
            Ok(())
        }
    }

    pub fn select_random_entries<F>(&mut self, n: usize, pass: bool, mut selector: F) -> Vec<usize>
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
        let result: Vec<usize> = borrows.iter().map(|x: &TreeBorrow| x.idx).collect();
        for i in borrows {
            tree.assign(i.idx, i.val);
        }
        result
    }

    pub fn set(&mut self, idx: usize, pass: bool) {
        let entry = &mut self.entries[idx];
        let dt0 = entry.distrust;
        const SMOOTH_F: f64 = 0.5;
        if entry.pass {
            self.cnt_failed += 1;
        }
        if pass {
            self.cnt_failed -= 1;
        }
        entry.pass = pass;
        entry.distrust = if pass {
            (dt0 + 1) / 2
        } else {
            let a: f64 = ((dt0 as f64) / (UNIT as f64)).powf(SMOOTH_F);
            ((UNIT as f64) * a) as i64
        };
        self.tree_passed.assign(idx, if pass { dt0 } else { 0 });
        self.tree_failed.assign(idx, if !pass { dt0 } else { 0 });
    }

    pub fn step(&mut self) {
        const DEGRADE_FACTOR: f64 = 0.8;
        const MINPREC: i64 = 100;
        let n = self.entries.len() as f64;
        let smult: f64 = DEGRADE_FACTOR.powf(n.recip());
        self.stp *= smult;
        if self.stp < MINPREC as f64 {
            const MULT: i64 = UNIT / MINPREC;
            self.tree_passed.multiply(MULT);
            self.tree_failed.multiply(MULT);
            self.stp *= smult;
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProgressEntry {
    /// Variable size from 0 to UNIT
    pub(crate) distrust: i64,
    pub(crate) pass: bool,
}
