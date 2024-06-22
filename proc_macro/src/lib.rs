use __exit::exit;

use __init::init;
use proc_macro::TokenStream;
mod __init;
mod __exit;

#[proc_macro_attribute]
pub fn __init(attr: TokenStream, item: TokenStream) -> TokenStream
{
    init(attr, item)
}

#[proc_macro_attribute]
pub fn __exit(attr: TokenStream, item: TokenStream) -> TokenStream
{
    exit(attr, item)
}