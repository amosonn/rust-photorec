use std::env::args_os;
use std::fs::File;

use photorec::{ReportXml, ReportXmlError, FileDescriptionError};

fn main() {
    let reports = args_os().skip(1).map(|fname| {
        let lossy = fname.to_string_lossy();
        println!("Parsing file {0}", &lossy);
        let f = File::open(&fname).expect(&lossy);
        let report = ReportXml::parse(f).expect(&lossy);
        (lossy.into_owned(), report)
    }).collect::<Vec<_>>();
    for (fname, report) in reports.iter() {
        let size = report.iter().filter_map(|x| {
            match x {
                Ok((s, fd)) => if s.ends_with(".jpg") { Some(fd.size()) } else { None }
                Err(ReportXmlError::BadFileDescription { file_name: ref s, source: ref e }) => {
                    if s.ends_with(".jpg") { match e {
                        FileDescriptionError::Empty => None,
                        _ => { x.unwrap(); unreachable!() } // We panic anyway
                    } } else { None } 
                }
                _ => { x.unwrap(); unreachable!() } // We panic anyway
            }
        }).fold(0, |a, b| a + b);
        let count = report.iter().filter_map(|x| {
            match x {
                Ok((s, _)) => if s.ends_with(".jpg") { Some(()) } else { None }
                Err(ReportXmlError::BadFileDescription { file_name: ref s, source: ref e }) => {
                    if s.ends_with(".jpg") { match e {
                        FileDescriptionError::Empty => None,
                        _ => { x.unwrap(); unreachable!() } // We panic anyway
                    } } else { None } 
                }
                _ => { x.unwrap(); unreachable!() } // We panic anyway
            }
        }).count();
        println!("{}: {} entries, {} bytes", fname, count, size);
    }
}
