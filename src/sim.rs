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
use rand::prelude::*;
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

pub enum UiMessageLegacy<'a> {
    Assess(&'a TableEntry, &'a mut String),
    Display(&'a TableEntry),
    NotifyAssessment,
}

pub enum UiMessage<'a> {
    Assess(&'a TableEntry),
    Display(&'a TableEntry),
    NotifyAssessment,
}

pub struct Simulation<'a> {
    pub pt: ProgressTable<'a>,
    pub args: SimArgs,
    state: State,
    last_msg: Option<UiMessage<'a>>,
    v1: Vec<(usize, &'a TableEntry)>,
    v2: Vec<(usize, &'a TableEntry)>,
    last_entry: Option<(usize, &'a TableEntry)>,
}

enum State {
    Begin,
    Assessment,
    Learning(LearningState),
    NotifyAssessment,
}

enum LearningState {
    ShowEntry,
    Assess,
}

impl<'a> Simulation<'a> {
    pub fn new(pt: ProgressTable<'a>, args: SimArgs) -> Simulation<'a> {
        Simulation {
            pt,
            args,
            state: State::Begin,
            last_msg: None,
            last_entry: None,
            v1: Vec::new(),
            v2: Vec::new(),
        }
    }

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
        F: Fn(UiMessageLegacy),
    {
        ffn(UiMessageLegacy::Display(ent.1));
        if self.args.classic {
            self.ptset(ent.0, true);
        }
    }

    fn assess_entry<F>(&mut self, ent: (usize, &TableEntry), ffn: &F) -> bool
    where
        F: Fn(UiMessageLegacy),
    {
        let mut ans = String::new();
        ffn(UiMessageLegacy::Assess(ent.1, &mut ans));
        let rpass = ent.1.assess(ans);
        self.ptset(ent.0, rpass);
        self.pt.step();
        rpass
    }

    fn next_inner(&mut self, post: &Option<String>) -> UiMessage {
        const LEARN_SESSIONS: usize = 10;
        const ASSESS_SESSIONS: usize = 10;
        let mut rng = thread_rng();

        loop {
            let (msg_opt, nstate): (Option<UiMessage>, State) = match &mut self.state {
                State::Begin => {
                    self.v1 = self
                        .pt
                        .select_random_entries(ASSESS_SESSIONS, true, || rng.gen::<f64>());
                    (None, State::Assessment)
                }
                State::Assessment => match &mut self.v1.pop() {
                    None => {
                        self.v1 = self
                            .pt
                            .select_random_entries(LEARN_SESSIONS, false, || 0_f64);
                        (None, State::Learning(LearningState::ShowEntry))
                    }
                    Some(tail) => {
                        self.last_entry = Some(*tail);
                        (Some(UiMessage::Assess(tail.1)), State::Assessment)
                    }
                },
                State::Learning(LearningState::ShowEntry) => match &mut self.v1.pop() {
                    None => (None, State::NotifyAssessment),
                    Some(entry) => {
                        if !self.args.classic {
                            self.v2 = vec![*entry]
                        } else {
                            self.v2.clear()
                        }
                        (
                            Some(UiMessage::Display(entry.1)),
                            State::Learning(LearningState::Assess),
                        )
                    }
                },
                State::Learning(LearningState::Assess) => {
                    match self.last_msg {
                        Some(UiMessage::Display(_)) => {
                            self.v2.extend(
                                self.pt
                                    .select_random_entries(1, true, || rng.gen::<f64>())
                                    .iter(),
                            );
                        }
                        Some(UiMessage::Assess(_)) => {
                            let tpost = post.clone().unwrap();
                            if !self.last_entry.unwrap().1.assess(tpost.clone()) {
                                self.v2.extend(
                                    self.pt
                                        .select_random_entries(1, true, || {
                                            thread_rng().gen::<f64>()
                                        })
                                        .iter(),
                                );
                                if !self.args.classic {
                                    self.v2.push(self.last_entry.unwrap());
                                }
                            }
                        }
                        _ => {
                            panic!();
                        }
                    }
                    match &mut self.v2.pop() {
                        None => (None, State::NotifyAssessment),
                        Some(tail) => {
                            self.last_entry = Some(*tail);
                            (
                                Some(UiMessage::Assess(tail.1)),
                                State::Learning(LearningState::Assess),
                            )
                        }
                    }
                }
                State::NotifyAssessment => {
                    self.v1 = self
                        .pt
                        .select_random_entries(ASSESS_SESSIONS, true, || rng.gen::<f64>());
                    (Some(UiMessage::NotifyAssessment), State::Assessment)
                }
            };
            self.state = nstate;
            if let Some(msg) = msg_opt {
                return msg;
            }
        }
    }

    pub fn next(&mut self, post: Option<String>) -> UiMessage {
        assert_eq!(
            matches!(self.last_msg, Some(UiMessage::Assess(_))),
            post.is_some()
        );

        self.next_inner(&post)
    }

    pub fn simulate<F>(&mut self, uimsg: &F)
    where
        F: Fn(UiMessageLegacy),
    {
        const LEARN_SESSIONS: usize = 10;
        const ASSESS_SESSIONS: usize = 10;
        loop {
            // State::
            // Assessment ( rentries: Vec<(usize, &TableEntry)> )
            let rentries = self
                .pt
                .select_random_entries(ASSESS_SESSIONS, true, || thread_rng().gen::<f64>());
            for rentry in rentries {
                self.assess_entry(rentry, uimsg);
            }
            // State::
            // Learning ( lentries: Vec<(usize, &TableEntry), ssa: LearningState )
            let lentries = self
                .pt
                .select_random_entries(LEARN_SESSIONS, false, || 0_f64);
            for lentry in lentries {
                // LearningState::
                // ShowEntry
                self.show_entry(lentry, uimsg);

                // LearningState::
                // Assess( stack : Vec<(usize, &TableEntry)>)
                let mut rep = VecDeque::<(usize, &TableEntry)>::new();
                if !self.args.classic {
                    rep.push_back(lentry);
                }
                rep.extend(
                    self.pt
                        .select_random_entries(1, true, || thread_rng().gen::<f64>())
                        .iter(),
                );
                while let Some(en) = rep.pop_front() {
                    if !self.assess_entry(en, uimsg) {
                        rep.extend(
                            self.pt
                                .select_random_entries(1, true, || thread_rng().gen::<f64>())
                                .iter(),
                        );
                        if !self.args.classic {
                            rep.push_back(en);
                        }
                        self.show_entry(en, uimsg);
                    }
                }
            }
            // State::
            // NotifyAssessment
            uimsg(UiMessageLegacy::NotifyAssessment);
        }
    }
}
