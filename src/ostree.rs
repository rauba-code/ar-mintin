#[derive(Debug)]
pub struct OSTree {
    arr: Vec<i64>,
}

impl OSTree {
    pub fn new(n: usize) -> OSTree {
        OSTree {
            arr: {
                let mut c: usize = 1;
                while c < n {
                    c *= 2;
                }
                vec![0; c * 2]
            },
        }
    }

    pub fn sum(&self) -> i64 {
        self.arr[1]
    }

    fn add(&mut self, mut idx: usize, sum: i64) {
        idx += self.arr.len() / 2;
        while idx > 0 {
            self.arr[idx] += sum;
            idx /= 2;
        }
    }

    pub fn value_at(&self, at: usize) -> i64 {
        self.arr[at + (self.arr.len() / 2)]
    }

    pub fn assign(&mut self, idx: usize, val: i64) {
        self.add(idx, val - self.value_at(idx))
    }

    pub fn rank(&self, mut val: i64) -> usize {
        let c: usize = self.arr.len();
        let mut p: usize = 2;
        while p < c {
            if self.arr[p] <= val {
                val -= self.arr[p];
                p += 1;
            }
            p *= 2;
        }
        (p - c) / 2
    }

    pub fn multiply(&mut self, coef: i64) {
        self.arr.iter_mut().for_each(|x| *x *= coef)
    }
}
