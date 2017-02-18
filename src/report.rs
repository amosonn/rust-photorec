//
// A parser, from photorec report.xml to a container of all file descriptions
// in it, including implementation for "opening" a file so.
//
use std::io::Read;
use std::fmt;
use std::error::Error;
use std::vec;

use xmltree::{Element, ParseError};

pub struct ReportXml {
    image_filename: String,
    iter: vec::IntoIter<Element>,
}


#[derive(Debug)]
pub enum ReportXmlError {
    Parse(ParseError),
    MissingField(String),
    MalformedText(String),
}

impl fmt::Display for ReportXmlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReportXmlError::Parse(ref x) => write!(f, "Error parsing: {}", x),
            ReportXmlError::MissingField(ref x) => write!(f, "Missing field {} in xml", x),
            ReportXmlError::MalformedText(ref x) => write!(f, "Malformed text in field {} in xml", x),
        }
    }
}

impl Error for ReportXmlError {
    fn description(&self) -> &str {
        match *self {
            ReportXmlError::Parse(ref x) => x.description(),
            ReportXmlError::MissingField(_) => "Missing field in xml",
            ReportXmlError::MalformedText(_) => "Malformed text in field in xml",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            ReportXmlError::Parse(ref x) => Some(x),
            ReportXmlError::MissingField(_) => None,
            ReportXmlError::MalformedText(_) => None,
        }
    }
}

impl From<ParseError> for ReportXmlError {
    fn from(pe: ParseError) -> Self { ReportXmlError::Parse(pe) }
}


macro_rules! try_child {
    ( $elem:expr, $name:expr ) => {
        { try!($elem.get_child($name).ok_or(ReportXmlError::MissingField($name.to_string()))) }
    }
}

macro_rules! try_text {
    ( $elem:expr ) => {
        { try!($elem.text.clone().ok_or(ReportXmlError::MalformedText($elem.name.clone()))) }
    }
}


impl ReportXml {
    pub fn parse<R: Read>(reader: R) -> Result<Self, ReportXmlError> {
        let elem = try!(Element::parse(reader));
        let elem = try!(elem.children.into_iter()
            .find(|e| e.name == "dfxml")
            .ok_or(ReportXmlError::MissingField("dfxml".to_string())));
        let image_filename = {
            let source = try_child!(elem, "source");
            let source = try_child!(source, "image_filename");
            try_text!(source)
        };
        Ok(ReportXml {
            image_filename: image_filename,
            iter: elem.children.into_iter(),
        })
    }
    
    pub fn image_filename(&self) -> &String { &self.image_filename }
}
