
use std::env::args;
use std::fs::File;
use std::fmt::{Formatter, Error as FmtError, Display};

use photorec::{SegmentArrayTree, SegmentArrayTreeError, ReportXml, FileDescription, ByteRun};

#[derive(Debug)]
struct FileDescriptionWithContext<'a> {
    desc: FileDescription,
    xml_name: &'a str,
    desc_name: String,
}

impl<'a> Display for FileDescriptionWithContext<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        f.write_str(self.xml_name)?;
        f.write_str(":")?;
        f.write_str(self.desc_name.as_ref())
    }
}

impl<'a> AsRef<[ByteRun]> for FileDescriptionWithContext<'a> {
    fn as_ref(&self) -> &[ByteRun] { self.desc.as_ref() }
}

fn main() {
    let mut sats = vec![SegmentArrayTree::new()];
    for (fname, report) in args().skip(1).map(|fname| {
        println!("Parsing file {0}", &fname);
        let f = File::open(&fname).expect(&fname);
        let report = ReportXml::parse(f).expect(&fname);
        (fname, report)
    }).collect::<Vec<_>>().iter() {
        println!();
        println!("Adding file {0}", &fname);
        for r in report.iter() {
            match r {
                Ok((name, desc)) => {
                    if !name.ends_with(".jpg") { continue; }
                    let mut fdwc = FileDescriptionWithContext {
                        desc,
                        xml_name: fname.as_ref(),
                        desc_name: name,
                    };
                    let mut add_new_tree = false;
                    let len = sats.len();
                    for (num, sat) in sats.iter_mut().enumerate() {
                        if num == len {
                            add_new_tree = true;
                        }
                        if let Err((_fdwc, e)) = sat.add(fdwc) {
                            fdwc = _fdwc;
                            let (fdwc1, fdwc2) = match e {
                                SegmentArrayTreeError::IntersectingSegment(idx) =>
                                    (sat.get_by_idx(idx), None),
                                SegmentArrayTreeError::OverlappingSegmentArrays(idx1, idx2) =>
                                    (sat.get_by_idx(idx1), Some(sat.get_by_idx(idx2))),
                                SegmentArrayTreeError::IncompatibleSegmentArrays(idx) =>
                                    (sat.get_by_idx(idx), None),
                            };
                            if let Some(fdwc2) = fdwc2 {
                                println!("On tree {num}, got error {e}, with relevant file descriptions at {0}, {1}, {2}", fdwc, fdwc1, fdwc2, e = e, num = num);
                            } else {
                                println!("On tree {num}, got error {e}, with relevant file descriptions at {0}, {1}", fdwc, fdwc1, e = e, num = num);
                            };
                        } else { break; }
                    }

                    if add_new_tree {
                        sats.push(SegmentArrayTree::new());
                    }
                }
                Err(e) => {
                    println!("At {0}: {1}", &fname, e);
                }
            }
        }
    }
}
