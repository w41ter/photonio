//! Procedural macros for PhotonIO.

#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;

/// Marks a function to be run on a runtime.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    transform(attr, item, false)
}

/// Marks a function to be run on a runtime for tests.
#[proc_macro_attribute]
pub fn test(attr: TokenStream, item: TokenStream) -> TokenStream {
    transform(attr, item, true)
}

fn transform(attr: TokenStream, item: TokenStream, is_test: bool) -> TokenStream {
    let opts = match Options::parse(attr.clone()) {
        Ok(opts) => opts,
        Err(e) => return token_stream_with_error(attr, e),
    };
    let mut func: syn::ItemFn = match syn::parse(item.clone()) {
        Ok(func) => func,
        Err(e) => return token_stream_with_error(item, e),
    };

    let mut rt = quote! {
        photonio::runtime::Builder::new()
    };
    if let Some(v) = opts.num_threads {
        rt = quote! { #rt.num_threads(#v) }
    }

    func.sig.asyncness = None;
    let block = func.block;
    func.block = syn::parse2(quote! {
        {
            let block = async #block;
            #rt.build().expect("failed to build runtime").block_on(block)
        }
    })
    .unwrap();

    let head = if is_test {
        quote! { #[::std::prelude::v1::test] }
    } else {
        quote! {}
    };

    quote! {
        #head
        #func
    }
    .into()
}

#[derive(Default)]
struct Options {
    num_threads: Option<usize>,
}

type Attributes = syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]>;

impl Options {
    fn parse(input: TokenStream) -> Result<Self, syn::Error> {
        let mut opts = Options::default();
        let attrs = Attributes::parse_terminated.parse(input)?;
        for attr in attrs {
            let name = attr
                .path
                .get_ident()
                .ok_or_else(|| syn::Error::new_spanned(&attr, "missing attribute name"))?
                .to_string();
            match name.as_str() {
                "num_threads" => {
                    opts.num_threads = Some(parse_int(&attr.lit)?);
                }
                _ => return Err(syn::Error::new_spanned(&attr, "unknown attribute name")),
            }
        }
        Ok(opts)
    }
}

fn parse_int(lit: &syn::Lit) -> Result<usize, syn::Error> {
    if let syn::Lit::Int(i) = lit {
        if let Ok(v) = i.base10_parse() {
            return Ok(v);
        }
    }
    Err(syn::Error::new(lit.span(), "failed to parse int"))
}

fn token_stream_with_error(mut item: TokenStream, error: syn::Error) -> TokenStream {
    item.extend(TokenStream::from(error.into_compile_error()));
    item
}
