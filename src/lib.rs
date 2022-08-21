/*
 * lib.rs -- Memorising application library
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

extern crate json;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate typed_arena;

pub mod ent;
pub mod ent_ex;
pub mod file;
pub mod file_ex;
mod ostree;
pub mod sim;
pub mod sim_ex;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
