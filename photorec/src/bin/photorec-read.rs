use std::env::args_os;
use std::fs::{File, create_dir, OpenOptions};
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::ffi::OsStr;

use photorec::{ReportXml, ByteRunsReader, Desc};

fn main() {
    let mut it = args_os().skip(1);
    // let temp = it.next_back().unwrap();
    let temp = it.next().unwrap();
    let output_dir = Path::new(&temp);
    let temp = it.next().unwrap();
    let volume_fname = Path::new(&temp);
    let volume = File::open(volume_fname).unwrap();
    let reports = it.map(|fname| {
        let lossy = fname.to_string_lossy();
        println!("Parsing file {0}", &lossy);
        let fname = PathBuf::new().join(&fname);
        let f = File::open(&fname).expect(&lossy);
        let report = ReportXml::parse(f).expect(&lossy);
        (fname, report)
    }).collect::<Vec<_>>();
    for (fname, report) in reports.into_iter() {
        let output_sub_dir = output_dir.join(&fname.file_stem().unwrap());
        println!("Creating dir {:?}", &output_sub_dir);
        create_dir(&output_sub_dir).unwrap();
        for r in report.iter() {
            match r {
                Ok((name, desc)) => {
                    let name = Path::new(&name);
                    if name.extension() != Some(OsStr::new("jpg")) { continue; }
                    let output_file = output_sub_dir.join(name.file_name().unwrap());
                    println!("Writing file {:?}", &output_file);
                    let mut file = OpenOptions::new().write(true).create_new(true).open(output_file).unwrap();
                    let mut brr = ByteRunsReader::new(&volume, desc.at_pos(0));
                    let mut buf = [0; 1024];
                    loop {
                        let x = brr.read(&mut buf).unwrap();
                        if x == 0 { break; }
                        file.write(&buf[..x]).unwrap();
                    }
                }
                Err(e) => {
                    println!("At {0}: {1}", fname.display(), e);
                }
            }
        }
    }
}
