use proc_macro::TokenStream;
use syn::{parse_macro_input, parse_quote, Attribute, ItemFn, Path};
use quote::{quote, ToTokens};

pub fn init(_attr: TokenStream, _func: TokenStream) -> TokenStream
{
    let mut func = parse_macro_input!(_func as ItemFn);
    func.attrs.push(parse_quote!(#[link_section = ".init.text"]));
    func.to_token_stream().into()
}