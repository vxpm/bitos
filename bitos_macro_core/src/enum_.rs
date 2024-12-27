use crate::common::BitosAttr;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident};
use syn::{Error, Expr, Ident, ItemEnum, Type, Variant, parse_quote_spanned, spanned::Spanned};

struct EnumVariant {
    _span: Span,
    ident: Ident,
    _value: Option<Expr>,
}

impl EnumVariant {
    fn new(variant: &Variant) -> Result<Self, Error> {
        let span = variant.span();
        let ident = variant.ident.clone();
        let value = variant.discriminant.as_ref().map(|(_, e)| e.clone());

        Ok(Self {
            _span: span,
            ident,
            _value: value,
        })
    }
}

pub struct BitEnum {
    pub def: ItemEnum,
    pub impl_: TokenStream,
}

impl BitEnum {
    pub fn new(bitos_attr: BitosAttr, e: ItemEnum) -> Result<Self, Error> {
        let inner_ty_name = format_ident!("u{}", bitos_attr.bitlen);
        let inner_ty: Box<Type> =
            Box::new(parse_quote_spanned! { bitos_attr.span => ::bitos::integer::#inner_ty_name });

        let mut variants = Vec::new();
        let variants_err =
            e.variants
                .iter()
                .map(EnumVariant::new)
                .fold(None, |acc: Option<Error>, r| match r {
                    Ok(f) => {
                        variants.push(f);
                        acc
                    }
                    Err(e) => {
                        if let Some(mut acc) = acc {
                            acc.combine(e);
                            Some(acc)
                        } else {
                            Some(e)
                        }
                    }
                });

        if let Some(e) = variants_err {
            return Err(e);
        }

        let variant_idents = variants.iter().map(|v| &v.ident).collect::<Vec<_>>();
        let variant_const_idents = variants
            .iter()
            .map(|v| format_ident!("CONST_{}", v.ident))
            .collect::<Vec<_>>();

        let ident = &e.ident;
        let generics = &e.generics;
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let bits_impl = (2usize.pow(bitos_attr.bitlen as u32) == e.variants.len()).then(|| {
            quote::quote! {
                impl #impl_generics ::bitos::Bits for #ident #ty_generics #where_clause {
                    #[inline(always)]
                    fn from_bits(value: Self::Bits) -> Self {
                        unsafe { <Self as ::bitos::TryBits>::try_from_bits(value).unwrap_unchecked() }
                    }
                }
            }
        });

        let impl_ = quote::quote! {
            impl #impl_generics ::bitos::TryBits for #ident #ty_generics #where_clause {
                type Bits = #inner_ty;

                #[inline(always)]
                #[allow(non_upper_case_globals)]
                fn try_from_bits(value: Self::Bits) -> ::core::option::Option<Self> {
                    #(
                        const #variant_const_idents: u64 = #ident::#variant_idents as u64;
                    )*

                    match <Self::Bits as ::bitos::integer::UnsignedInt>::value(value) {
                        #(
                            #variant_const_idents => Some(Self::#variant_idents),
                        )*
                        _ => None,
                    }
                }

                #[inline(always)]
                fn into_bits(self) -> Self::Bits {
                    <Self::Bits as ::bitos::integer::UnsignedInt>::new(self as u64)
                }
            }

            #bits_impl
        };

        Ok(BitEnum { def: e, impl_ })
    }
}

impl ToTokens for BitEnum {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { def, impl_ } = self;

        tokens.extend(quote::quote! {
            #def
            #impl_
        });
    }
}
