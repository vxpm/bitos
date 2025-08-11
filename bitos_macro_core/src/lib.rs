mod common;
mod enum_;
mod struct_;

use common::BitosAttr;
use quote::ToTokens;
use syn::{Error, Item, parse2, spanned::Spanned};

pub fn bitos_attr(
    attr: proc_macro2::TokenStream,
    input: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, Error> {
    let input: Item = parse2(input)?;
    let bity_attr: BitosAttr = parse2(attr)?;

    match input {
        Item::Struct(s) => struct_::BitStruct::new(bity_attr, s).map(ToTokens::into_token_stream),
        Item::Enum(e) => enum_::BitEnum::new(bity_attr, e).map(ToTokens::into_token_stream),
        _ => Err(Error::new(input.span(), "Unsupported item")),
    }
}
