
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRun {
    file_offset: u64,
    disk_pos: u64,
    len: u64,
}

pub struct ByteRunsRef {
    runs: Box<[ByteRun]>,
    size: u64,
    pos: u64,
    cur_run: usize,
    offset_in_run: u64,
}

impl ByteRunsRef {
    pub fn new<T>(size: u64, it: T) -> Self where T: Iterator<Item=ByteRun> {
        let mut runs: Vec<ByteRun> = it.collect();
        let gross_size = &runs.iter().map(|br| br.len).sum();
        runs.split_last_mut().unwrap().0.len -= gross_size - size;
        runs.sort();
        ByteRunsRef {
            runs: runs.into_boxed_slice(),
            size: size,
            pos: 0,
            cur_run: 0,
            offset_in_run: 0,
        }
    }
}
