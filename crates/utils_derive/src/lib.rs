extern crate proc_macro;
mod utils;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(DocReader)]
pub fn derive_doc_reader(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    utils::doc_reader::extract_derive_doc_reader(&mut input)
}
