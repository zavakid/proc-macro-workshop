mod builder;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
use crate::builder::BuilderContext;

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    //println!("{:#?}", input);

    let builder_context = BuilderContext::new(input);
    builder_context.generate().into()

    //TokenStream::default()
}