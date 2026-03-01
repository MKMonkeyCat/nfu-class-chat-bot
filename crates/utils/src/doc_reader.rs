use std::collections::HashMap;

use indexmap::IndexMap;
pub use utils_derive::DocReader;

#[derive(Debug, Clone)]
pub enum DocNode {
    Leaf(&'static str),
    Node(&'static str, IndexMap<&'static str, DocNode>),
}

pub trait DocReader {
    fn get_struct_doc() -> &'static str;
    fn get_field_docs() -> IndexMap<&'static str, &'static str>;
    fn get_recursive_docs() -> DocNode;
}

pub trait DocReaderInternal {
    fn get_node() -> DocNode;
}

macro_rules! impl_leaf {
    ($($t:ty),*) => {
        $(impl DocReaderInternal for $t {
            fn get_node() -> DocNode { DocNode::Leaf("") }
        })*
    };
}

impl_leaf!(
    String, &str, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64, bool
);

impl<T: DocReaderInternal> DocReaderInternal for Vec<T> {
    fn get_node() -> DocNode {
        T::get_node()
    }
}

impl<K, V: DocReaderInternal> DocReaderInternal for HashMap<K, V> {
    fn get_node() -> DocNode {
        let mut children: indexmap::IndexMap<&'static str, DocNode> = indexmap::IndexMap::new();
        children.insert("*", V::get_node());
        DocNode::Node("", children)
    }
}

impl<T: DocReaderInternal> DocReaderInternal for Option<T> {
    fn get_node() -> DocNode {
        T::get_node()
    }
}
