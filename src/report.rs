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

type Result<T> = std::result::Result<T, ReportXmlError>;

#[derive(Error, Debug)]
pub enum ReportXmlError {
    #[error("Error parsing: {0}")]
    Parse(#[from] ParseError),
    #[error("Missing field {0} in xml")]
    MissingField(&'static str),
    #[error("Missing text in field {field_name} in xml")]
    MissingText { field_name: String },
    #[error("Missing attr {attr_name} in field {field_name} in xml")]
    MissingAttr { attr_name: &'static str, field_name: String },
    #[error("Malformed text in field {field_name} in xml, parse error: {source}")]
    MalformedText { field_name: String, #[source] source: num::ParseIntError },
    #[error("Malformed attr {attr_name} in field {field_name} in xml, parse error: {source}")]
    MalformedAttr { attr_name: &'static str, field_name: String, #[source] source: num::ParseIntError },
    #[error("File {file_name} has a bad ByteRunsRef: {source}")]
    BadByteRunsRef { file_name: String, #[source] source: ByteRunsRefError },
}

fn get_child<'a>(elem: &'a Element, name: &'static str) -> Result<&'a Element> {
    elem.get_child(name).ok_or(ReportXmlError::MissingField(name))
}

fn get_text<'a>(elem: &'a Element) -> Result<&'a String> {
    elem.text.as_ref().ok_or(ReportXmlError::MissingText { field_name: elem.name.clone() })
}

fn get_number<'a>(elem: &'a Element) -> Result<u64> {
    let x = get_text(elem)?;
    x.parse::<u64>().map_err(|e| ReportXmlError::MalformedText { field_name: elem.name.clone(), source: e })
}

fn get_attr_number<'a>(elem: &'a Element, name: &'static str) -> Result<u64> {
    let x = elem.attributes.get(name).ok_or(ReportXmlError::MissingAttr { attr_name: name, field_name: elem.name.clone() })?;
    x.parse::<u64>().map_err(|e| ReportXmlError::MalformedAttr { attr_name: name, field_name: elem.name.clone(), source: e })
}

fn to_byte_runs_ref(elem: Element) -> Result<(String, ByteRunsRef)> {
    let name = get_text(get_child(&elem, "filename")?)?.clone();
    let size = get_number(get_child(&elem, "filesize")?)?;
    let byte_runs = get_child(&elem, "byte_runs")?.children.iter()
        .map(|x| -> Result<ByteRun> {
            let file_offset = get_attr_number(x, "offset")?;
            let disk_pos = get_attr_number(x, "img_offset")?;
            let len = get_attr_number(x, "len")?;
            Ok(ByteRun { file_offset, disk_pos, len })
        }).collect::<Result<Vec<ByteRun>>>()?;
    let byte_runs_ref = ByteRunsRef::new(size, byte_runs)
        .map_err(|e| ReportXmlError::BadByteRunsRef { file_name: name.clone(), source: e })?;
    Ok((name, byte_runs_ref))
}

impl ReportXml {
    pub fn parse<R: Read>(reader: R) -> Result<Self> {
        let elem = Element::parse(reader)?;
        let image_filename = {
            let source = get_child(&elem, "source")?;
            let source = get_child(source, "image_filename")?;
            get_text(source)?
        };
        Ok(ReportXml {
            image_filename: image_filename.clone(),
            iter: elem.children.into_iter(),
        })
    }
    
    pub fn image_filename(&self) -> &String { &self.image_filename }
}

impl Iterator for ReportXml {
    type Item = Result<(String, ByteRunsRef)>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|ref x| x.name == "fileobject").map(to_byte_runs_ref)
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
    assert_let!(ReportXmlError::BadByteRunsRef { file_name: x, source: e } = e, {
        assert_eq!(x, "f140247350_assets.zip");
        assert_let!(ByteRunsRefError::Empty = e);
    });
    let e = rx.next().unwrap().unwrap();
    assert_eq!(e.0, "f140197124_res.zip");
    assert!(rx.next().is_none());
}
