/*
 * sim.rs -- Simulation configuration
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

use crate::args;
use crate::cli;
use crate::ent::ProgressTable;
use crate::ent::TableEntry;
use std::collections::VecDeque;
use std::io::prelude::*;

pub struct Simulation<'a> {
    pub pt: ProgressTable<'a>,
    pub args: args::Args,
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
        if self.args.classic {
            self.ptset(ent.0, true);
        }
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
                let mut rep = VecDeque::<(usize, &TableEntry)>::new();
                if !self.args.classic {
                    rep.push_back(lentry);
                }
                rep.extend(self.pt.select_random_entries(1, true, &mut selector).iter());
                while let Some(en) = rep.pop_front() {
                    if !self.assess_entry(en, lines) {
                        rep.extend(self.pt.select_random_entries(1, true, &mut selector).iter());
                        if !self.args.classic {
                            rep.push_back(en);
                        }
                        self.show_entry(en, lines);
                    }
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
