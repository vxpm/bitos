#[proc_macro_attribute]
pub fn bitos(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match bitos_macro_core::bitos_attr(attr.into(), input.into()) {
        Ok(x) => x.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
