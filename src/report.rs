//
// A parser, from photorec report.xml to a container of all file descriptions
// in it, including implementation for "opening" a file so.
//
use std::io::Read;
use std::vec;
use std::num;

use thiserror::Error;

use xmltree::{Element, ParseError};

use super::byte_runs::{ByteRun, ByteRunsRef, ByteRunsRefError};

pub struct ReportXml {
    image_filename: String,
    iter: vec::IntoIter<Element>,
}


#[derive(Error, Debug)]
pub enum ReportXmlError {
    #[error("Error parsing: {0}")]
    Parse(#[from] ParseError),
    #[error("Missing field {0} in xml")]
    MissingField(String),
    #[error("Missing text in field {0} in xml")]
    MissingText(String),
    #[error("Missing attr {0} in field <missing> in xml")]
    MissingAttr(String),
    #[error("Malformed text in field {0} in xml, parse error: {1}")]
    MalformedText(String, #[source] num::ParseIntError),
    #[error("Malformed attr {0} in field <missing> in xml, parse error: {1}")]
    MalformedAttr(String, #[source] num::ParseIntError),
    #[error("File {0} has a bad ByteRunsRef: {1}")]
    BadByteRunsRef(String, #[source] ByteRunsRefError),
}


macro_rules! try_child {
    ( $elem:expr, $name:expr ) => {
        { $elem.get_child($name).ok_or(ReportXmlError::MissingField($name.to_string()))? }
    }
}

macro_rules! try_text {
    ( $elem:expr ) => {
        { 
            let elem = $elem;
            elem.text.as_ref().ok_or(ReportXmlError::MissingText(elem.name.clone()))?
        }
    }
}

macro_rules! try_number {
    ( $elem:expr ) => {
        { 
            let elem = $elem;
            let x = elem.text.as_ref().ok_or(ReportXmlError::MissingText(elem.name.clone()))?;
            x.parse::<u64>().map_err(|e| ReportXmlError::MalformedText(elem.name.clone(), e))?
        }
    }
}

macro_rules! try_attr_number {
    ( $elem:expr, $name:expr ) => {
        { 
            let x = $elem.attributes.get($name).ok_or(ReportXmlError::MissingAttr($name.to_string()))?;
            x.parse::<u64>().map_err(|e| ReportXmlError::MalformedAttr($name.to_string(), e))?
        }
    }
}


impl ReportXml {
    pub fn parse<R: Read>(reader: R) -> Result<Self, ReportXmlError> {
        let elem = Element::parse(reader)?;
        let image_filename = {
            let source = try_child!(elem, "source");
            let source = try_child!(source, "image_filename");
            try_text!(source).clone()
        };
        Ok(ReportXml {
            image_filename: image_filename,
            iter: elem.children.into_iter(),
        })
    }
    
    pub fn image_filename(&self) -> &String { &self.image_filename }
}

macro_rules! get {
    ($expr:expr) => (match $expr {
        Option::Some(val) => val,
        Option::None => { return Option::None }
    })
}

impl Iterator for ReportXml {
    type Item = Result<(String, ByteRunsRef), ReportXmlError>;
    fn next(&mut self) -> Option<Self::Item> {
        let elem = get!(self.iter.find(|ref x| x.name == "fileobject"));
        fn inner(elem: Element) -> Result<(String, ByteRunsRef), ReportXmlError> {
            let name = try_text!(try_child!(elem, "filename")).clone();
            let size = try_number!(try_child!(elem, "filesize"));
            let byte_runs = try_child!(elem, "byte_runs").children.iter()
                .map(|x| -> Result<ByteRun, ReportXmlError> {
                    Ok(ByteRun {
                        file_offset: try_attr_number!(x, "offset"),
                        disk_pos: try_attr_number!(x, "img_offset"),
                        len: try_attr_number!(x, "len"),
                    })
                }).collect::<Result<Vec<ByteRun>, ReportXmlError>>()?;
            let byte_runs_ref = ByteRunsRef::new(size, byte_runs)
                .map_err(|e| ReportXmlError::BadByteRunsRef(name.clone(), e))?;
            Ok((name, byte_runs_ref))
        }
        Some(inner(elem))
    }
}

#[test]
fn test_report_xml_parse() {
    let s = r##"<?xml version='1.0' encoding='UTF-8'?>
<dfxml xmloutputversion='1.0'>
  <metadata 
  xmlns='http://www.forensicswiki.org/wiki/Category:Digital_Forensics_XML' 
  xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' 
  xmlns:dc='http://purl.org/dc/elements/1.1/'>
    <dc:type>Carve Report</dc:type>
  </metadata>
  <creator>
    <package>PhotoRec</package>
    <version>7.1-WIP</version>
    <build_environment>
      <compiler>GCC 6.2</compiler>
      <library name='libext2fs' version='1.2.3'/>
      <library name='libewf' version='none'/>
      <library name='libjpeg' version='libjpeg-turbo-1.2.3'/>
      <library name='libntfs' version='libntfs-3g'/>
      <library name='zlib' version='1.2.3'/>
    </build_environment>
    <execution_environment>
      <os_sysname>Linux</os_sysname>
      <os_release>4.5.6</os_release>
      <os_version>Version</os_version>
      <host>test</host>
      <arch>x86_64</arch>
      <uid>0</uid>
      <start_time>2017-02-19T02:02:21+0100</start_time>
    </execution_environment>
  </creator>
  <source>
    <image_filename>/dev/sdb</image_filename>
    <sectorsize>512</sectorsize>
    <device_model>Generic STORAGE DEVICE</device_model>
    <image_size>123456000000</image_size>
    <volume>
      <byte_runs>
        <byte_run offset='0' img_offset='16777216' len='123439222784'/>
      </byte_runs>
    </volume>
  </source>
  <configuration>
  </configuration>
  <fileobject>
    <filename>f140247350_assets.zip</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='71823420416' len='10167808'/>
      <byte_run offset='10167808' img_offset='71833914368' len='4608'/>
      <byte_run offset='10172416' img_offset='71833920512' len='321024'/>
      <byte_run offset='10493440' img_offset='71835273216' len='6144'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f140247350_assets.zip</filename>
    <filesize>10499571</filesize>
    <byte_runs>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f140197124_res.zip</filename>
    <filesize>31628</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='71797704704' len='19456'/>
      <byte_run offset='19456' img_offset='71798924800' len='12288'/>
    </byte_runs>
  </fileobject>
</dfxml>"##;
    let mut rx = ReportXml::parse(s.as_bytes()).unwrap();
    assert_eq!(rx.image_filename(), "/dev/sdb");
    let e = rx.next().unwrap().unwrap();
    assert_eq!(e.0, "f140247350_assets.zip");
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::BadByteRunsRef(x, e) = e, {
        assert_eq!(x, "f140247350_assets.zip");
        assert_let!(ByteRunsRefError::Empty = e);
    });
    let e = rx.next().unwrap().unwrap();
    assert_eq!(e.0, "f140197124_res.zip");
    assert!(rx.next().is_none());
}
