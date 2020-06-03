#[macro_use]
mod tests;

mod byte_runs;
mod reader;
mod report;

pub use crate::byte_runs::{ByteRun, ByteRunsRef, ByteRunsRefPos, ByteRunsRefError};
pub use crate::reader::ByteRunsReader;
pub use crate::report::{ReportXml, ReportXmlError};
