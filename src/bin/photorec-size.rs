use std::env::args_os;
use std::fs::File;

use photorec::ReportXml;

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
            let (s, fd) = x.unwrap();
            if s.ends_with(".jpg") { Some(fd.size()) } else { None }
        }).fold(0, |a, b| a + b);
        let count = report.iter().filter_map(|x| {
            let (s, fd) = x.unwrap();
            if s.ends_with(".jpg") { Some(fd.size()) } else { None }
        }).count();
        println!("{}: {} entries, {} bytes", fname, count, size);
    }
}
