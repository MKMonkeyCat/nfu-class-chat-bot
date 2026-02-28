extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Lit, Meta, parse_macro_input};

#[proc_macro_derive(DocReader)]
pub fn derive_doc_reader(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let struct_docs = extract_docs(&input.attrs);

    let field_entries = match &input.data {
        Data::Struct(data) => collect_fields_docs_list(&data.fields),
        Data::Enum(data) => data
            .variants
            .iter()
            .map(|v| {
                let name = v.ident.to_string();
                let docs = extract_docs(&v.attrs);
                quote! { (#name, #docs) }
            })
            .collect(),
        _ => vec![],
    };

    let recursive_items = match &input.data {
        Data::Struct(data) => {
            data.fields.iter().enumerate().map(|(i, field)| {
                let name = field.ident.as_ref()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| i.to_string());

                let field_ty = &field.ty;
                let field_docs = extract_docs(&field.attrs);

                quote! {
                    {
                        let node = <#field_ty as doc_reader::DocReaderInternal>::get_node();
                        let final_node = match node {
                            doc_reader::DocNode::Leaf(_) => doc_reader::DocNode::Leaf(#field_docs),
                            doc_reader::DocNode::Node(_, children) => doc_reader::DocNode::Node(#field_docs, children),
                        };
                        map.insert(#name, final_node);
                    }
                }
            }).collect::<Vec<_>>()
        }
        _ => vec![],
    };

    let expanded = quote! {
        impl #impl_generics doc_reader::DocReader for #struct_name #ty_generics #where_clause {
            fn get_struct_doc() -> &'static str {
                #struct_docs
            }

            fn get_field_docs() -> indexmap::IndexMap<&'static str, &'static str> {
                let mut map = indexmap::IndexMap::new();
                let entries: &[(&'static str, &'static str)] = &[ #(#field_entries),* ];
                for &(k, v) in entries {
                    map.insert(k, v);
                }
                map
            }

            fn get_recursive_docs() -> doc_reader::DocNode {
                let mut map = indexmap::IndexMap::new();
                #(#recursive_items)*
                doc_reader::DocNode::Node(#struct_docs, map)
            }
        }

        impl #impl_generics doc_reader::DocReaderInternal for #struct_name #ty_generics #where_clause {
            fn get_node() -> doc_reader::DocNode {
                Self::get_recursive_docs()
            }
        }
    };

    TokenStream::from(expanded)
}

fn collect_fields_docs_list(fields: &Fields) -> Vec<proc_macro2::TokenStream> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let name = field
                .ident
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_else(|| i.to_string());
            let docs = extract_docs(&field.attrs);
            quote! { (#name, #docs) }
        })
        .collect()
}

fn extract_docs(attrs: &[Attribute]) -> String {
    let content = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            if let Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    return Some(s.value());
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n");
    content.trim().to_string()
}
