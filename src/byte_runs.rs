
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRun {
    file_offset: u64,
    disk_pos: u64,
    len: u64,
}

pub struct ByteRunsRef {
    runs: Box<[ByteRun]>,
    pos: u64,
    cur_run: usize,
    offset_in_run: u64,
}

impl ByteRunsRef {
    pub fn new<T>(it: T) -> Self where T: Iterator<Item=ByteRun> {
        let mut runs: Vec<ByteRun> = it.collect();
        runs.sort();
        ByteRunsRef {
            runs: runs.into_boxed_slice(),
            pos: 0,
            cur_run: 0,
            offset_in_run: 0,
        }
    }
}
