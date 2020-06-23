
use std::env::args_os;
use std::fs::File;
use std::fmt::{Formatter, Error as FmtError, Display};
use std::{path::Path, iter::FromIterator};

use photorec::{SegmentArrayTree, SegmentArrayTreeError, ReportXml, FileDescription, ByteRun, AddStatus};

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
    let mut it = args_os().skip(1);
    // let temp = it.next_back().unwrap();
    let temp = it.next().unwrap();
    let output_dir = Path::new(&temp);
    let reports = it.map(|fname| {
        let lossy = fname.to_string_lossy();
        println!("Parsing file {0}", &lossy);
        let f = File::open(&fname).expect(&lossy);
        let report = ReportXml::parse(f).expect(&lossy);
        (lossy.into_owned(), report)
    }).collect::<Vec<_>>();
    for (fname, report) in reports.iter() {
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
                    let last = sats.len() - 1;
                    for (num, sat) in sats.iter_mut().enumerate() {
                        if num == last {
                            add_new_tree = true;
                        }
                        match sat.add(fdwc) {
                            Err((_fdwc, e)) => {
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
                            } 
                            Ok(AddStatus::Replaced(fdwc1)) => {
                                println!("On tree {num}, replaced file description at {fdwc}", num = num, fdwc = fdwc1);
                                break;
                            }
                            _ => { break; }
                        }
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
    for (num, sat) in sats.into_iter().enumerate() {
        let output_path = output_dir.join(format!("report{}.xml", num));
        let f = File::create(output_path).unwrap();
        let rx = ReportXml::from_iter(sat.into_iter().map(|fdwc| (fdwc.desc_name, fdwc.desc)));
        rx.write(f).unwrap();
    }
}
