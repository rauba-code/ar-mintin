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
use std::io::Read;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score(pub i64);

impl Score {
    /// Score function: Finds the unit score from the given age.
    /// The main variable here is 'arg'.
    /// The variable 'inertia' controls how slowly the curve degrades.
    /// It is usually set to the size of the memory list.
    /// Variables inside 'ScoreArgs' are intended to be static.
    pub fn function(age: i32, inertia: f64, sa: &ScoreArgs) -> f64 {
        if sa.origin == sa.target {
            sa.origin.0 as f64
        } else {
            // TODO: write unit tests for the math
            let u = (sa.origin.0 as f64).ln();
            let v = (sa.target.0 as f64).ln();
            let a = age as f64;
            let phi = sa.degrade_factor.ln();
            let epoch = (-(u - v) / phi) * inertia;
            let mage = epoch * ((a / epoch) - (a / epoch).floor());
            (u + (mage * phi / inertia)).exp()
        }
    }

    /// Finds the age from the unit score.
    /// May return `Null` if the argument is out of range.
    /// The function does not round the result.
    pub fn inverse(unit: f64, inertia: f64, sa: &ScoreArgs) -> Option<f64> {
        let u = sa.origin.0 as f64;
        let v = sa.target.0 as f64;
        if u.min(v) <= unit && unit <= u.max(v) {
            Some(inertia * (unit / u).log(sa.degrade_factor))
        } else {
            None
        }
    }
}

/// Static arguments to compute the score unit funtion.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ScoreArgs {
    pub degrade_factor: f64,
    pub origin: Score,
    pub target: Score,
}

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
struct ProgressTableViewLegacy {
    entries: Vec<(ProgressEntry, TableEntry)>,
    stp: f64,
}

#[derive(Serialize, Deserialize)]
pub struct ProgressTableView {
    pub entries: Vec<(ProgressEntry, TableEntry)>,
    pub age: i32,
    pub score_args: ScoreArgs,
}

impl ProgressTableView {
    pub fn new(table: &ProgressTable, te: &[TableEntry]) -> ProgressTableView {
        ProgressTableView {
            age: table.age,
            score_args: table.score_args,
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
    pub(crate) entries: Vec<ProgressEntry>,
    capacity: usize,
    cnt_failed: usize,
    tree_passed: OSTree,
    tree_failed: OSTree,
    age: i32,
    score_args: ScoreArgs,
}

pub struct UnitConstants {}

pub const UNIT: Score = Score(10000);

pub struct OutOfRangeError;

impl ProgressTable {
    fn tree_from_entries(entries: &[ProgressEntry], pass: bool) -> OSTree {
        let mut tree = OSTree::new(entries.len());
        for (idx, &entry) in entries.iter().enumerate() {
            if entry.pass == pass {
                tree.assign(idx, entry.distrust.0);
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

    pub fn unit_score(&self) -> Score {
        Score(Score::function(self.age, self.entries.len() as f64, &self.score_args) as i64)
    }

    pub fn get_age(&self) -> i32 {
        self.age
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn migrate(buf: &[u8]) -> ProgressTableView {
        let data: ProgressTableViewLegacy = serde_json::from_slice(buf).unwrap();
        const SCORE_ARGS: ScoreArgs = ScoreArgs {
            degrade_factor: 0.8,
            origin: Score(10000),
            target: Score(100),
        };
        ProgressTableView {
            score_args: SCORE_ARGS,
            age: Score::inverse(data.stp, data.entries.len() as f64, &SCORE_ARGS).unwrap() as i32,
            entries: data.entries,
        }
    }

    pub fn new_from_file(entries: &[TableEntry], path: &Path) -> ProgressTable {
        use std::collections::HashMap;
        let mut buf = Vec::<u8>::new();
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        let data: ProgressTableView =
            serde_json::from_slice(&buf).unwrap_or_else(|_| Self::migrate(&buf));
        let mut imap = HashMap::new();
        for entry in data.entries {
            imap.insert(entry.1, entry.0);
        }
        let n = entries.len();
        let pe = || {
            entries.iter().map(|x| {
                if imap.contains_key(x) {
                    imap[x]
                } else {
                    ProgressEntry {
                        distrust: Score(
                            Score::function(data.age, n as f64, &data.score_args) as i64
                        ),
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
            age: data.age,
            score_args: data.score_args,
        }
    }

    pub fn new(entries: Pin<Arc<Vec<TableEntry>>>, score_args: ScoreArgs) -> ProgressTable {
        let n = entries.len();
        Self::new_partial(entries, n, 0, score_args)
    }

    pub fn new_empty(capacity: usize, age: i32, score_args: ScoreArgs) -> ProgressTable {
        ProgressTable {
            entries: Vec::new(),
            capacity,
            cnt_failed: 0,
            tree_passed: OSTree::new(capacity),
            tree_failed: OSTree::new(capacity),
            age,
            score_args,
        }
    }

    pub fn new_partial(
        entries: Pin<Arc<Vec<TableEntry>>>,
        capacity: usize,
        age: i32,
        score_args: ScoreArgs,
    ) -> ProgressTable {
        let n = entries.len();
        let unit = Score(Score::function(age, n as f64, &score_args) as i64);
        ProgressTable {
            entries: vec![
                ProgressEntry {
                    distrust: unit,
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
                    xt.assign(i, unit.0);
                }
                xt
            },
            age,
            score_args,
        }
    }

    pub fn supply(&mut self, chunk: &[ProgressEntry]) -> Result<(), OutOfRangeError> {
        let m = chunk.len();
        let n = self.entries.len();
        if n + m > self.capacity {
            Err(OutOfRangeError)
        } else {
            for (i, pe) in chunk.iter().enumerate() {
                if pe.pass {
                    self.tree_passed.assign(i + n, pe.distrust.0);
                } else {
                    self.tree_failed.assign(i + n, pe.distrust.0);
                    self.cnt_failed += 1;
                }
            }
            self.entries.extend(chunk);

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
        const SMOOTH_F: f64 = 0.5;
        let us = self.unit_score().0 as f64;
        let entry = &mut self.entries[idx];
        let dt0 = entry.distrust;
        if entry.pass {
            self.cnt_failed += 1;
        }
        if pass {
            self.cnt_failed -= 1;
        }
        entry.pass = pass;
        entry.distrust = if pass {
            Score((dt0.0 + 1) / 2)
        } else {
            let a: f64 = ((dt0.0 as f64) / us).powf(SMOOTH_F);
            Score((us * a) as i64)
        };
        self.tree_passed.assign(idx, if pass { dt0.0 } else { 0 });
        self.tree_failed.assign(idx, if !pass { dt0.0 } else { 0 });
    }

    pub fn step(&mut self) {
        self.age += 1
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ProgressEntry {
    /// Variable size from 0 to UNIT
    pub distrust: Score,
    pub pass: bool,
}
