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

#[derive(Clone, Debug)]
pub enum TMessage<T> {
    Assess(T),
    Display(T),
    NotifyAssessment,
}

pub type UiMessage<'a> = TMessage<&'a TableEntry>;

pub struct Simulation<'a> {
    pub pt: ProgressTable<'a>,
    pub args: SimArgs,
    last_msg: Option<TMessage<Tie<'a>>>,
    state: Main<'a>,
}

impl<'a> Simulation<'a> {
    pub fn new(pt: ProgressTable<'a>, args: SimArgs) -> Simulation<'a> {
        Simulation {
            pt,
            args,
            last_msg: None,
            state: Main::new(),
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

    pub fn next(&mut self, post: Option<String>) -> UiMessage {
        assert_eq!(
            matches!(self.last_msg, Some(TMessage::Assess(_))),
            post.is_some()
        );

        let pass = if let Some(TMessage::Assess(ent)) = self.last_msg {
            let b = ent.1.assess(post.unwrap());
            self.ptset(ent.0, b);
            b
        } else {
            false
        };
        let inp = &mut Input {
            pt: &mut self.pt,
            args: &mut self.args,
        };

        let r = self.state.next(inp, pass, 1);
        eprintln!();
        eprintln!();
        self.last_msg = r.clone();
        match r.unwrap() {
            TMessage::Assess(a) => UiMessage::Assess(a.1),
            TMessage::Display(a) => UiMessage::Display(a.1),
            TMessage::NotifyAssessment => UiMessage::NotifyAssessment,
        }
    }

    #[warn(deprecated)]
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

pub struct Input<'a, 'b> {
    pt: &'b mut ProgressTable<'a>,
    args: &'b mut SimArgs,
}

const MAXDEPTH: u16 = 30;
pub trait Domain<'a> {
    fn next<'b>(
        &mut self,
        inp: &mut Input<'a, 'b>,
        pass: bool,
        depth: u16,
    ) -> Option<TMessage<Tie<'a>>>
    where
        'a: 'b;
}

#[derive(Debug)]
pub enum Bivariant<T, U> {
    V1(T),
    V2(U),
}

#[derive(Debug, Default)]
pub struct Main<'a> {
    inner: Option<Bivariant<Assessment<'a>, Learning<'a>>>,
}

impl<'a> Main<'a> {
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl<'a> Domain<'a> for Main<'a> {
    fn next<'b>(
        &mut self,
        inp: &mut Input<'a, 'b>,
        pass: bool,
        depth: u16,
    ) -> Option<TMessage<Tie<'a>>>
    where
        'a: 'b,
    {
        assert!(depth < MAXDEPTH);
        let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
        eprintln!("{} {:?}", indent, self);
        let r = match &mut self.inner {
            None => {
                self.inner = Some(Bivariant::V1(Assessment::new(inp)));
                self.next(inp, pass, depth + 1)
            }
            Some(Bivariant::V1(a)) => match a.next(inp, pass, depth + 1) {
                None => {
                    self.inner = Some(Bivariant::V2(Learning::new(inp)));
                    self.next(inp, pass, depth + 1)
                }
                Some(b) => Some(b),
            },
            Some(Bivariant::V2(a)) => match a.next(inp, pass, depth + 1) {
                None => {
                    self.inner = Some(Bivariant::V1(Assessment::new(inp)));
                    self.next(inp, pass, depth + 1)
                }
                Some(b) => Some(b),
            },
        };
        eprintln!("{} {:?}", indent, r);
        r
    }
}

type Tie<'a> = (usize, &'a TableEntry);

#[derive(Debug)]
pub struct Assessment<'a> {
    began: bool,
    ents: Vec<Tie<'a>>,
}

impl<'a> Assessment<'a> {
    pub fn new<'b>(inp: &mut Input<'a, 'b>) -> Self {
        const ASSESS_SESSIONS: usize = 10;
        let mut rng = thread_rng();
        let ents = inp
            .pt
            .select_random_entries(ASSESS_SESSIONS, true, || rng.gen::<f64>());
        Self { ents, began: false }
    }
}

impl<'a> Domain<'a> for Assessment<'a> {
    fn next<'b>(
        &mut self,
        _: &mut Input<'a, 'b>,
        _pass: bool,
        depth: u16,
    ) -> Option<TMessage<Tie<'a>>>
    where
        'a: 'b,
    {
        assert!(depth < MAXDEPTH);
        let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
        eprintln!("{} {:?}", indent, self);
        let r = {
            if self.began {
                self.ents.pop().map(TMessage::Assess)
            } else {
                self.began = true;
                if self.ents.is_empty() {
                    None
                } else {
                    Some(TMessage::NotifyAssessment)
                }
            }
        };
        eprintln!("{} {:?}", indent, r);
        r
    }
}

#[derive(Debug)]
pub struct Learning<'a> {
    ents: Vec<Tie<'a>>,
    inner: Option<LearnSingle<'a>>,
}

impl<'a> Learning<'a> {
    pub fn new<'b>(inp: &mut Input<'a, 'b>) -> Self {
        const LEARN_SESSIONS: usize = 10;
        let mut ents = inp
            .pt
            .select_random_entries(LEARN_SESSIONS, false, || 0_f64);
        ents.reverse();
        Self { ents, inner: None }
    }
}

impl<'a> Domain<'a> for Learning<'a> {
    fn next<'b>(
        &mut self,
        inp: &mut Input<'a, 'b>,
        _pass: bool,
        depth: u16,
    ) -> Option<TMessage<Tie<'a>>>
    where
        'a: 'b,
    {
        assert!(depth < MAXDEPTH);
        let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
        eprintln!("{} {:?}", indent, self);
        let r = match &mut self.inner {
            None => self.ents.pop().and_then(|tail| {
                self.inner = Some(LearnSingle::new(tail));
                self.next(inp, _pass, depth + 1)
            }),
            Some(ls) => ls.next(inp, _pass, depth + 1).or_else(|| {
                self.inner = None;
                self.next(inp, _pass, depth + 1)
            }),
        };
        eprintln!("{} {:?}", indent, r);
        r
    }
}

#[derive(Debug)]
pub struct LearnSingle<'a> {
    began: bool,
    head: Option<Tie<'a>>,
    stack: Vec<Tie<'a>>,
}

impl<'a> LearnSingle<'a> {
    pub fn new(head: Tie<'a>) -> Self {
        Self {
            began: false,
            stack: Vec::new(),
            head: Some(head),
        }
    }
}

impl<'a> Domain<'a> for LearnSingle<'a> {
    fn next<'b>(
        &mut self,
        inp: &mut Input<'a, 'b>,
        pass: bool,
        depth: u16,
    ) -> Option<TMessage<Tie<'a>>>
    where
        'a: 'b,
    {
        assert!(depth < MAXDEPTH);
        let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
        eprintln!("{} {:?}", indent, self);
        let r = if let Some(vhead) = self.head.filter(|_| !pass || !self.began) {
            self.began = true;
            self.head = None;
            if !inp.args.classic {
                self.stack.push(vhead);
            }
            self.stack.extend(
                inp.pt
                    .select_random_entries(1, true, || thread_rng().gen::<f64>())
                    .iter(),
            );
            Some(TMessage::Display(vhead))
        } else {
            self.stack.pop().map(|tail| {
                self.head = Some(tail);
                TMessage::Assess(tail)
            })
        };
        eprintln!("{} {:?}", indent, r);
        r
    }
}
