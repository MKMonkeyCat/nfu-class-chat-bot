#[cfg(feature = "doc_reader")]
mod doc_reader;
#[cfg(feature = "template_string")]
mod template_string;

#[cfg(feature = "doc_reader")]
pub use doc_reader::*;

#[cfg(feature = "template_string")]
pub use template_string::*;
