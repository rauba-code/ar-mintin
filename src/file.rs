/*
 * file.rs -- Utility functions for file IO
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

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::ent::TableEntry;

pub fn load_table(path: &Path) -> Vec<TableEntry> {
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
