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

use crate::ent::ProgressTable;
use crate::ent::TableEntry;
use std::collections::VecDeque;

use std::path::PathBuf;

pub struct SimArgs {
    /// Path to the progress file.
    /// If the specified file does not exist,
    ///   a new file is attempted to be created on the path.
    /// Otherwise, the given file is read.
    /// If the flag is not specified, the progress is not tracked.
    pub progress: Option<PathBuf>,

    /// Output path to the progress file
    /// If the path is not specified,
    ///   the output path is read from --progress path.
    pub outprogress: Option<PathBuf>,

    /// Simulate classic mode
    /// (no rehearsal of the learned sentence)
    pub classic: bool,
}

pub enum UiMessage<'a> {
    Assess(&'a TableEntry, &'a mut String),
    Display(&'a TableEntry),
    NotifyAssessment,
}

pub struct Simulation<'a> {
    pub pt: ProgressTable<'a>,
    pub args: SimArgs,
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

    fn show_entry<F>(&mut self, ent: (usize, &TableEntry), ffn: &F)
    where
        F: Fn(UiMessage),
    {
        ffn(UiMessage::Display(ent.1));
        if self.args.classic {
            self.ptset(ent.0, true);
        }
    }

    fn assess_entry<F>(&mut self, ent: (usize, &TableEntry), ffn: &F) -> bool
    where
        F: Fn(UiMessage),
    {
        let mut ans = String::new();
        ffn(UiMessage::Assess(ent.1, &mut ans));
        let rpass = ent.1.assess(ans);
        self.ptset(ent.0, rpass);
        self.pt.step();
        rpass
    }

    pub fn simulate<F>(&mut self, uimsg: &F)
    where
        F: Fn(UiMessage),
    {
        use rand::prelude::*;
        const LEARN_SESSIONS: usize = 10;
        const ASSESS_SESSIONS: usize = 10;
        let mut rng = rand::thread_rng();
        let mut selector = || rng.gen::<f64>();
        loop {
            let rentries = self
                .pt
                .select_random_entries(ASSESS_SESSIONS, true, &mut selector);
            for rentry in rentries {
                self.assess_entry(rentry, uimsg);
            }
            let lentries = self
                .pt
                .select_random_entries(LEARN_SESSIONS, false, || 0_f64);
            for lentry in lentries {
                self.show_entry(lentry, uimsg);
                let mut rep = VecDeque::<(usize, &TableEntry)>::new();
                if !self.args.classic {
                    rep.push_back(lentry);
                }
                rep.extend(self.pt.select_random_entries(1, true, &mut selector).iter());
                while let Some(en) = rep.pop_front() {
                    if !self.assess_entry(en, uimsg) {
                        rep.extend(self.pt.select_random_entries(1, true, &mut selector).iter());
                        if !self.args.classic {
                            rep.push_back(en);
                        }
                        self.show_entry(en, uimsg);
                    }
                }
            }
            uimsg(UiMessage::NotifyAssessment);
        }
    }
}
