/*
 * cli.rs -- Utility functions for manipulating the command line
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

use crossterm::{cursor, terminal, ExecutableCommand};
use std::io::{stdout, Lines, StdinLock};
pub fn cls() {
    stdout()
        .lock()
        .execute(terminal::Clear(terminal::ClearType::FromCursorUp))
        .unwrap()
        .execute(cursor::MoveTo(0, 1))
        .unwrap();
}

pub fn standby(lines: &mut Lines<StdinLock>) {
    stdout().lock().execute(cursor::Hide).unwrap();
    if let Some(x) = lines.next() {
        x.unwrap();
    }
    stdout().lock().execute(cursor::Show).unwrap();
    cls();
}

pub fn readin(lines: &mut std::io::Lines<std::io::StdinLock>) -> Option<String> {
    std::io::stdout()
        .lock()
        .execute(cursor::MoveRight(4))
        .unwrap();
    let r = lines.next().map(|x| x.unwrap());
    cls();
    r
}
