extern crate json;

use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
struct TableEntry {
    lhs: String,
    rhs: String,
}

fn load_table(path: &Path) -> Vec<TableEntry> {
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

struct ProgressTable<'a> {
    entries: Vec<(ProgressEntry, &'a TableEntry)>,
}

struct ProgressEntry {
    /// Variable size from 0 to 1
    trust: f64,
    pass: bool,
}

struct OSTree {
    arr: Vec<f64>,
}

impl OSTree {
    pub fn new(n: usize) -> OSTree {
        OSTree {
            arr: {
                let mut c: usize = 1;
                while c < n {
                    c *= 2;
                }
                c *= 2;
                let mut v = Vec::<f64>::with_capacity(c);
                v.resize(c, 0.0);
                v
            },
        }
    }

    pub fn add(&mut self, mut idx: usize, sum: f64) {
        idx += self.arr.len() / 2;
        while idx > 0 {
            self.arr[idx] += sum;
            idx /= 2;
        }
    }

    pub fn value_at(&self, at: usize) -> f64 {
        self.arr[at + (self.arr.len() / 2)]
    }

    pub fn assign(&mut self, idx: usize, val: f64) {
        self.add(idx, val - self.value_at(idx))
    }

    pub fn rank(&self, mut val: f64) {
        let mut p: usize = 2;
        while p < self.arr.len() {
            if self.arr[p] <= val {
                val -= self.arr[p];
                p += 1;
            }
            p *= 2;
        }
    }
}

fn main() {
    let inpath = Path::new("demo.json");
    let table: Vec<TableEntry> = load_table(inpath);
    println!("{:#?}", table);
}
