use crossterm::{cursor, terminal, ExecutableCommand};
use std::io::{stdout, Lines, StdinLock};
pub fn cls() {
    stdout()
        .lock()
        .execute(terminal::Clear(terminal::ClearType::All))
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
