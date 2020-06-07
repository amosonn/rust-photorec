//
// A parser, from photorec report.xml to a container of all file descriptions
// in it, including implementation for "opening" a file so.
//
use std::io::Read;
use std::vec;
use std::num;

use thiserror::Error;

use xmltree::{Element, ParseError};

use super::file_description::{ByteRun, FileDescription, FileDescriptionError};

#[derive(Debug)]
pub struct ReportXml {
    image_filename: String,
    iter: vec::IntoIter<Element>,
}

type Result<T> = std::result::Result<T, ReportXmlError>;

#[derive(Error, Debug)]
pub enum ReportXmlError {
    #[error("Error parsing: {0}")]
    Parse(#[from] ParseError),
    #[error("Missing field {field_name} in xml")]
    MissingField { field_name: &'static str },
    #[error("Missing text in field {field_name} in xml")]
    MissingText { field_name: String },
    #[error("Unexpected child of name {field_name} in xml")]
    BadChildName { expected_name: &'static str, field_name: String },
    #[error("Missing attr {attr_name} in field {field_name} in xml")]
    MissingAttr { attr_name: &'static str, field_name: String },
    #[error("Malformed text in field {field_name} in xml, parse error: {source}")]
    MalformedText { field_name: String, #[source] source: num::ParseIntError },
    #[error("Malformed attr {attr_name} in field {field_name} in xml, parse error: {source}")]
    MalformedAttr { attr_name: &'static str, field_name: String, #[source] source: num::ParseIntError },
    #[error("File {file_name} has a bad FileDescription: {source}")]
    BadFileDescription { file_name: String, #[source] source: FileDescriptionError },
}

fn get_child<'a>(elem: &'a Element, name: &'static str) -> Result<&'a Element> {
    elem.get_child(name).ok_or(ReportXmlError::MissingField { field_name: name })
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

fn assert_name<'a>(elem: &'a Element, name: &'static str) -> Result<()> {
    if elem.name != name {
        Err(ReportXmlError::BadChildName { expected_name: name, field_name: elem.name.clone() })
    } else {
        Ok(())
    }
}


fn to_file_description(elem: Element) -> Result<(String, FileDescription)> {
    let name = get_text(get_child(&elem, "filename")?)?.clone();
    let size = get_number(get_child(&elem, "filesize")?)?;
    let byte_runs = get_child(&elem, "byte_runs")?.children.iter()
        .map(|x| -> Result<ByteRun> {
            assert_name(x, "byte_run")?;
            let file_offset = get_attr_number(x, "offset")?;
            let disk_pos = get_attr_number(x, "img_offset")?;
            let len = get_attr_number(x, "len")?;
            Ok(ByteRun { file_offset, disk_pos, len })
        }).collect::<Result<Vec<ByteRun>>>()?;
    let file_description = FileDescription::new(size, byte_runs)
        .map_err(|e| ReportXmlError::BadFileDescription { file_name: name.clone(), source: e })?;
    Ok((name, file_description))
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
    type Item = Result<(String, FileDescription)>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find(|ref x| x.name == "fileobject").map(to_file_description)
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
    // We only check errors are handled and iteration continues
    let _e = rx.next().unwrap().err().unwrap();
    let e = rx.next().unwrap().unwrap();
    assert_eq!(e.0, "f140197124_res.zip");
    assert!(rx.next().is_none());
}

#[test]
fn test_report_xml_parse_errors() {
    let s = r##"<?xml version='1.0' encoding='UTF-8'?>
<dfxml xmloutputversion='1.0'"##;
    let rx_err = ReportXml::parse(s.as_bytes());
    assert_let!(Err(ReportXmlError::Parse(_)) = rx_err);

    let s = r##"<?xml version='1.0' encoding='UTF-8'?>
<dfxml xmloutputversion='1.0'>
</dfxml>"##;
    let rx_err = ReportXml::parse(s.as_bytes());
    assert_let!(Err(ReportXmlError::MissingField { field_name: ref s }) = rx_err, {
        assert_eq!(*s, "source");
    });

    let s = r##"<?xml version='1.0' encoding='UTF-8'?>
<dfxml xmloutputversion='1.0'>
  <source>
  </source>
</dfxml>"##;
    let rx_err = ReportXml::parse(s.as_bytes());
    assert_let!(Err(ReportXmlError::MissingField { field_name: ref s }) = rx_err, {
        assert_eq!(*s, "image_filename");
    });

    let s = r##"<?xml version='1.0' encoding='UTF-8'?>
<dfxml xmloutputversion='1.0'>
  <source>
    <image_filename />
  </source>
</dfxml>"##;
    let rx_err = ReportXml::parse(s.as_bytes());
    assert_let!(Err(ReportXmlError::MissingText { field_name: ref s }) = rx_err, {
        assert_eq!(*s, "image_filename");
    });
}

#[test]
fn test_report_xml_iter_errors() {
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
  <fileobject />
  <fileobject>
    <filesize>123</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='1' len='2'/>
    </byte_runs>
    <byte_runs />
  </fileobject>
  <fileobject>
    <filename>f1</filename>
    <byte_runs>
      <byte_run offset='0' img_offset='1' len='2'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f2</filename>
    <filesize>not-a-number</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='1' len='2'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f2</filename>
    <filesize>10499571</filesize>
  </fileobject>
  <fileobject>
    <filename>f3</filename>
    <filesize>10499571</filesize>
    <byte_runs />
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <bad_name />
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run img_offset='16777216' len='123439222784'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run offset='nan' img_offset='16777216' len='123439222784'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run offset='0' len='123439222784'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='nan' len='123439222784'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='16777216'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='16777216' len='nan'/>
    </byte_runs>
  </fileobject>
  <fileobject>
    <filename>f4</filename>
    <filesize>10499571</filesize>
    <byte_runs>
      <byte_run offset='0' img_offset='16777216' len='123439222784'/>
      <bad_name />
    </byte_runs>
  </fileobject>
</dfxml>"##;
    let mut rx = ReportXml::parse(s.as_bytes()).unwrap();
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MissingField { field_name: ref s } = e, {
        assert_eq!(*s, "filename");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MissingField { field_name: ref s } = e, {
        assert_eq!(*s, "filename");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MissingField { field_name: ref s } = e, {
        assert_eq!(*s, "filesize");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MalformedText { field_name: ref s, source: _ } = e, {
        assert_eq!(*s, "filesize");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MissingField { field_name: ref s } = e, {
        assert_eq!(*s, "byte_runs");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::BadFileDescription { file_name: x, source: e } = e, {
        assert_eq!(x, "f3");
        assert_let!(FileDescriptionError::Empty = e);
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::BadChildName { expected_name: ref exp, field_name: ref field } = e, {
        assert_eq!(*exp, "byte_run");
        assert_eq!(*field, "bad_name");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MissingAttr { attr_name: ref a, field_name: ref f } = e, {
        assert_eq!(*a, "offset");
        assert_eq!(*f, "byte_run");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MalformedAttr { attr_name: ref a, field_name: ref f, source: _ } = e, {
        assert_eq!(*a, "offset");
        assert_eq!(*f, "byte_run");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MissingAttr { attr_name: ref a, field_name: ref f } = e, {
        assert_eq!(*a, "img_offset");
        assert_eq!(*f, "byte_run");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MalformedAttr { attr_name: ref a, field_name: ref f, source: _ } = e, {
        assert_eq!(*a, "img_offset");
        assert_eq!(*f, "byte_run");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MissingAttr { attr_name: ref a, field_name: ref f } = e, {
        assert_eq!(*a, "len");
        assert_eq!(*f, "byte_run");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::MalformedAttr { attr_name: ref a, field_name: ref f, source: _ } = e, {
        assert_eq!(*a, "len");
        assert_eq!(*f, "byte_run");
    });
    let e = rx.next().unwrap().err().unwrap();
    assert_let!(ReportXmlError::BadChildName { expected_name: ref exp, field_name: ref field } = e, {
        assert_eq!(*exp, "byte_run");
        assert_eq!(*field, "bad_name");
    });
}
