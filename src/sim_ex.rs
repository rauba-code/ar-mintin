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

use crate::ent_ex::ProgressTable;
use crate::ent_ex::TableEntry;
use rand::prelude::*;

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

#[derive(Clone, Debug)]
pub enum TMessage<T> {
    Assess(T),
    Display(T),
    NotifyAssessment,
}

pub type UiMessage = TMessage<usize>;

pub struct Simulation {
    pub pt: ProgressTable,
    pub args: SimArgs,
    last_msg: Option<UiMessage>,
    state: Main,
}

impl Simulation {
    pub fn new(pt: ProgressTable, args: SimArgs) -> Simulation {
        Simulation {
            pt,
            args,
            last_msg: None,
            state: Main::new(),
        }
    }

    fn ptset(&mut self, te: &[TableEntry], idx: usize, val: bool) {
        self.pt.set(idx, val);
        if let Some(op) = self
            .args
            .outprogress
            .as_ref()
            .or(self.args.progress.as_ref())
        {
            self.pt.write_to_file(te, op)
        }
    }

    pub fn next(&mut self, te: &[TableEntry], post: Option<String>) -> UiMessage {
        assert_eq!(
            matches!(self.last_msg, Some(TMessage::Assess(_))),
            post.is_some()
        );

        let pass = if let Some(TMessage::Assess(ent)) = self.last_msg {
            let b = te[ent].assess(post.unwrap());
            self.ptset(te, ent, b);
            b
        } else {
            false
        };
        let inp = &mut Input {
            //et: te,
            pt: &mut self.pt,
            args: &self.args,
        };

        let r = self.state.next(inp, pass, 1);
        if cfg!(sim_debug) {
            eprintln!();
            eprintln!();
        }
        self.last_msg = r.clone();
        r.unwrap()
    }

    pub fn flush_state(&mut self) {
        self.state = Main::new();
    }
}

pub struct Input<'b> {
    //et: &'b Vec<TableEntry>,
    pt: &'b mut ProgressTable,
    args: &'b SimArgs,
}

const MAXDEPTH: u16 = 30;
pub trait Domain {
    fn next<'b>(&mut self, inp: &mut Input<'b>, pass: bool, depth: u16) -> Option<UiMessage>;
}

#[derive(Debug)]
pub enum Bivariant<T, U> {
    V1(T),
    V2(U),
}

#[derive(Debug, Default)]
pub struct Main {
    inner: Option<Bivariant<Assessment, Learning>>,
}

impl Main {
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl Domain for Main {
    fn next<'b>(&mut self, inp: &mut Input<'b>, pass: bool, depth: u16) -> Option<UiMessage> {
        assert!(depth < MAXDEPTH);
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, self);
        }
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
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, r);
        }
        r
    }
}

#[derive(Debug)]
pub struct Assessment {
    began: bool,
    ents: Vec<usize>,
}

impl Assessment {
    pub fn new(inp: &mut Input) -> Self {
        const ASSESS_SESSIONS: usize = 10;
        let mut rng = thread_rng();
        let ents = inp
            .pt
            .select_random_entries(ASSESS_SESSIONS, true, || rng.gen::<f64>());
        Self { ents, began: false }
    }
}

impl Domain for Assessment {
    fn next<'b>(&mut self, _: &mut Input<'b>, _pass: bool, depth: u16) -> Option<UiMessage> {
        assert!(depth < MAXDEPTH);
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, self);
        }
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
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, r);
        }
        r
    }
}

#[derive(Debug)]
pub struct Learning {
    ents: Vec<usize>,
    inner: Option<LearnSingle>,
}

impl Learning {
    pub fn new(inp: &mut Input) -> Self {
        const LEARN_SESSIONS: usize = 10;
        let mut ents = inp
            .pt
            .select_random_entries(LEARN_SESSIONS, false, || 0_f64);
        ents.reverse();
        Self { ents, inner: None }
    }
}

impl Domain for Learning {
    fn next<'b>(&mut self, inp: &mut Input<'b>, _pass: bool, depth: u16) -> Option<UiMessage> {
        assert!(depth < MAXDEPTH);
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, self);
        }
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
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, r);
        }
        r
    }
}

#[derive(Debug)]
pub struct LearnSingle {
    began: bool,
    head: Option<usize>,
    stack: Vec<usize>,
}

impl LearnSingle {
    pub fn new(head: usize) -> Self {
        Self {
            began: false,
            stack: Vec::new(),
            head: Some(head),
        }
    }
}

impl Domain for LearnSingle {
    fn next<'b>(&mut self, inp: &mut Input<'b>, pass: bool, depth: u16) -> Option<UiMessage> {
        assert!(depth < MAXDEPTH);
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, self);
        }
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
        if cfg!(sim_debug) {
            let indent = unsafe { String::from_utf8_unchecked(vec![b'|'; depth as usize]) };
            eprintln!("{} {:?}", indent, r);
        }
        r
    }
}
