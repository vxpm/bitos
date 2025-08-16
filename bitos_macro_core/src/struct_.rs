use std::ops::Range;

use crate::common::{BitosAttr, BitsAttr, extract_derive};
use heck::ToShoutySnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote_spanned};
use syn::{
    Attribute, Error, Expr, Field, Ident, ItemImpl, ItemStruct, Type, Visibility, parse_quote,
    parse_quote_spanned, spanned::Spanned,
};

enum FieldTy {
    Simple(Box<Type>),
    /// [T; N]
    Array {
        span: Span,
        elem: Box<Type>,
        len: Expr,
    },
    /// Option<T>
    Try(Box<Type>),
}

impl FieldTy {
    pub fn new(ty: &Type) -> Self {
        match ty {
            Type::Array(ty_arr) => FieldTy::Array {
                span: ty.span(),
                elem: ty_arr.elem.clone(),
                len: ty_arr.len.clone(),
            },
            Type::Path(ty_path) => {
                let paths: [&[&str]; 3] = [
                    &["std", "option", "Option"],
                    &["core", "option", "Option"],
                    &["Option"],
                ];

                for path in paths {
                    let is_path = ty_path
                        .path
                        .segments
                        .iter()
                        .zip(path.iter())
                        .all(|(ty_segment, path_segment)| ty_segment.ident == path_segment);

                    if !is_path {
                        continue;
                    }

                    let args = &ty_path.path.segments.last().unwrap().arguments;
                    if let syn::PathArguments::AngleBracketed(args) = args
                        && args.args.len() == 1
                    {
                        let arg = args.args.first().unwrap();
                        let syn::GenericArgument::Type(ty_arg) = arg else {
                            continue;
                        };

                        return FieldTy::Try(Box::new(ty_arg.clone()));
                    } else {
                        continue;
                    }
                }

                FieldTy::Simple(Box::new(ty.clone()))
            }
            _ => FieldTy::Simple(Box::new(ty.clone())),
        }
    }
}

impl ToTokens for FieldTy {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldTy::Simple(ty) => ty.to_tokens(tokens),
            FieldTy::Array { span, elem, len } => {
                tokens.extend(quote_spanned! { *span => [#elem; #len] })
            }
            FieldTy::Try(ty) => ty.to_tokens(tokens),
        }
    }
}

struct StructField {
    span: Span,
    vis: Visibility,
    ident: Ident,
    ty: FieldTy,
    bits: BitsAttr,
    docs: Vec<Attribute>,
}

impl StructField {
    fn new(field: &Field) -> Result<Self, Error> {
        let span = field.span();
        let vis = field.vis.clone();
        let ident = field
            .ident
            .clone()
            .ok_or(Error::new(span, "structs must have named fields"))?;
        let ty = FieldTy::new(&field.ty);

        let mut attrs = field.attrs.clone();
        let Some(bits) = BitsAttr::extract(&mut attrs)? else {
            return Err(Error::new(span, "field must have a #[bits(..)] attribute"));
        };

        let docs = attrs
            .extract_if(.., |a| a.meta.path().is_ident("doc"))
            .collect();

        Ok(Self {
            span,
            vis,
            ident,
            ty,
            bits,
            docs,
        })
    }

    fn bitrange(&self, bitstruct: &BitStructInput) -> Range<usize> {
        let bits_start = self.bits.bitrange.start();
        let bits_end = self
            .bits
            .bitrange
            .end()
            .unwrap_or(bitstruct.bitos_attr.bitlen);

        bits_start..bits_end
    }

    fn bitlen(&self) -> Expr {
        match &self.ty {
            FieldTy::Simple(ty) => {
                parse_quote_spanned! { ty.span() => <<#ty as ::bitos::TryBits>::Bits as ::bitos::integer::UnsignedInt>::BITS }
            }
            FieldTy::Array { span, elem, len } => {
                parse_quote_spanned! { *span => <<#elem as ::bitos::TryBits>::Bits as ::bitos::integer::UnsignedInt>::BITS * #len }
            }
            FieldTy::Try(ty) => {
                parse_quote_spanned! { ty.span() => <<#ty as ::bitos::TryBits>::Bits as ::bitos::integer::UnsignedInt>::BITS }
            }
        }
    }

    fn assertions(&self, bitstruct: &BitStructInput) -> Expr {
        let field_ty_bitlen = self.bitlen();
        let range = self.bitrange(bitstruct);
        let specified_bitlen = range.end.saturating_sub(range.start);
        let bitlen = bitstruct.bitos_attr.bitlen;
        let bitlen_msg = format!("field '{}' has wrong bit length", self.ident);

        let start_err = (range.start > bitlen).then(|| {
            Error::new(
                self.bits.span,
                format!(
                    "start of field '{}' is out of range: should be in 0..{}",
                    self.ident, bitlen
                ),
            )
            .into_compile_error()
        });
        let end_err = (range.end > bitlen).then(|| {
            Error::new(
                self.bits.span,
                format!(
                    "end of field '{}' is out of range: should be in 0..{}",
                    self.ident, bitlen
                ),
            )
            .into_compile_error()
        });

        parse_quote_spanned! {
            self.bits.span =>
            {
                #start_err
                #end_err
                assert!(#field_ty_bitlen == #specified_bitlen, #bitlen_msg);
            }
        }
    }

    fn mask(&self, bitstruct: &BitStructInput) -> Result<TokenStream, Error> {
        let Self {
            span,
            vis,
            ident,
            bits,
            ..
        } = self;

        let bits_start = bits.bitrange.start() as u8;
        let bits_end = bits.bitrange.end().unwrap_or(bitstruct.bitos_attr.bitlen) as u8;
        let len = bits_end.saturating_sub(bits_start);
        let mask_value = (((1u128 << len) - 1) as u64) << bits_start;
        let mask_value = mask_value & (((1u128 << bitstruct.bitos_attr.bitlen) - 1) as u64);

        let mask_ident = format_ident!("{}_MASK", ident.to_string().to_shouty_snake_case());
        let mask = quote::quote! { #mask_value as _ };

        Ok(quote_spanned! {
            *span =>
            #[doc = "Mask where only bits of the `"]
            #[doc = stringify!(#ident)]
            #[doc = "` field are set"]
            #vis const #mask_ident: u64 = #mask;
        })
    }

    fn getter(&self, bitstruct: &BitStructInput) -> Result<TokenStream, Error> {
        let Self {
            span,
            vis,
            ident,
            ty: field_ty,
            bits,
            docs,
        } = self;

        let bits_start = bits.bitrange.start() as u8;
        let bits_end = bits.bitrange.end().unwrap_or(bitstruct.bitos_attr.bitlen) as u8;

        let inner_ty = &bitstruct.inner_ty;
        let field_ident_str = ident.to_string();
        let field_getter_ident = format_ident!("{}", ident);

        match field_ty {
            FieldTy::Simple(field_ty) => Ok(quote_spanned! {
                *span =>
                #(#docs)*
                #[inline(always)]
                #vis fn #field_getter_ident (&self) -> #field_ty {
                    #[allow(unused_imports)]
                    use bitos::{TryBits, Bits, BitUtils, integer::UnsignedInt};
                    const { Self::__assertions() };

                    let extracted_bits = self.0.bits(#bits_start, #bits_end);
                    let extracted_downcast = <<#field_ty as TryBits>::Bits as UnsignedInt>::new(
                        <#inner_ty as UnsignedInt>::value(extracted_bits)
                    );

                    <#field_ty>::from_bits(extracted_downcast)
                }
            }),
            FieldTy::Array { elem, len, .. } => {
                let field_elem_getter_ident = format_ident!("{}_at", ident);

                Ok(quote_spanned! {
                    *span =>
                    #[doc = "Gets the element at the given index in the `"]
                    #[doc = #field_ident_str]
                    #[doc = "` field."]
                    #[inline(always)]
                    #vis fn #field_elem_getter_ident (&self, index: usize) -> ::core::option::Option<#elem> {
                        #[allow(unused_imports)]
                        use bitos::{TryBits, Bits, BitUtils, integer::UnsignedInt};
                        const { Self::__assertions() };

                        (index < #len).then(|| {
                            let elem_len = <#elem as TryBits>::Bits::BITS as u8;
                            let offset = #bits_start + elem_len * index as u8;
                            let extracted_bits = self.0.bits(offset, offset + elem_len);
                            let extracted_downcast = <<#elem as TryBits>::Bits as UnsignedInt>::new(
                                <#inner_ty as UnsignedInt>::value(extracted_bits)
                            );

                            <#elem>::from_bits(extracted_downcast)
                        })

                    }

                    #(#docs)*
                    #[inline(always)]
                    #vis fn #field_getter_ident (&self) -> #field_ty {
                        const { Self::__assertions() };
                        core::array::from_fn(|i| unsafe { self.#field_elem_getter_ident(i).unwrap_unchecked() })
                    }
                })
            }
            FieldTy::Try(field_ty) => Ok(quote_spanned! {
                *span =>
                #(#docs)*
                #[inline(always)]
                #vis fn #field_getter_ident (&self) -> ::core::option::Option<#field_ty> {
                    #[allow(unused_imports)]
                    use bitos::{TryBits, BitUtils, integer::UnsignedInt};
                    const { Self::__assertions() };

                    let extracted_bits = self.0.bits(#bits_start, #bits_end);
                    let extracted_downcast = <<#field_ty as TryBits>::Bits as UnsignedInt>::new(
                        <#inner_ty as UnsignedInt>::value(extracted_bits)
                    );

                    <#field_ty>::try_from_bits(extracted_downcast)
                }
            }),
        }
    }

    fn setters(&self, bitstruct: &BitStructInput) -> Result<TokenStream, Error> {
        let Self {
            span,
            vis,
            ident,
            ty: field_ty,
            bits,
            ..
        } = self;

        let bits_start = bits.bitrange.start() as u8;
        let bits_end = bits.bitrange.end().unwrap_or(bitstruct.bitos_attr.bitlen) as u8;

        let inner_ty = &bitstruct.inner_ty;
        let field_ident_str = ident.to_string();
        let field_setter_ident = format_ident!("set_{}", ident);
        let field_with_ident = format_ident!("with_{}", ident);

        match field_ty {
            FieldTy::Simple(field_ty) => Ok(quote_spanned! {
                *span =>
                #[doc = "Sets the value of the `"]
                #[doc = #field_ident_str]
                #[doc = "` field."]
                #[inline(always)]
                #vis fn #field_setter_ident (&mut self, value: #field_ty) -> &mut Self {
                    #[allow(unused_imports)]
                    use bitos::{TryBits, BitUtils, integer::UnsignedInt};
                    const { Self::__assertions() };

                    let value_bits = value.to_bits();
                    let value_upcast = <#inner_ty as UnsignedInt>::new(
                        <<#field_ty as TryBits>::Bits as UnsignedInt>::value(value_bits)
                    );

                    self.0 = self.0.with_bits(#bits_start, #bits_end, value_upcast);
                    self
                }

                #[doc = "Consumes `self` to modify the value of the `"]
                #[doc = #field_ident_str]
                #[doc = "` field and returns the modified `self`."]
                #[inline(always)]
                #vis fn #field_with_ident (mut self, value: #field_ty) -> Self {
                    self.#field_setter_ident(value);
                    self
                }
            }),
            FieldTy::Array { elem, len, .. } => {
                let field_elem_setter_ident = format_ident!("set_{}_at", ident);
                let field_elem_with_ident = format_ident!("with_{}_at", ident);

                Ok(quote_spanned! {
                    *span =>
                    #[doc = "Sets a single element in the `"]
                    #[doc = #field_ident_str]
                    #[doc = "` field."]
                    #[inline(always)]
                    #vis fn #field_elem_setter_ident (&mut self, index: usize, value: #elem) -> &mut Self {
                        #[allow(unused_imports)]
                        use bitos::{TryBits, BitUtils, integer::UnsignedInt};
                        const { Self::__assertions() };

                        if index < #len {
                            let elem_len = <#elem as TryBits>::Bits::BITS as u8;
                            let offset = #bits_start + elem_len * index as u8;

                            let value_bits = value.to_bits();
                            let value_upcast = <#inner_ty as UnsignedInt>::new(
                                <<#elem as TryBits>::Bits as UnsignedInt>::value(value_bits)
                            );

                            self.0 = self.0.with_bits(offset, offset + elem_len, value_upcast);
                        }

                        self
                    }

                    #[doc = "Consumes `self` to modify the value of a element in the `"]
                    #[doc = #field_ident_str]
                    #[doc = "` field and returns the modified `self`."]
                    #[inline(always)]
                    #vis fn #field_elem_with_ident (mut self, index: usize, value: #elem) -> Self {
                        self.#field_elem_setter_ident(index, value);
                        self
                    }

                    #[doc = "Sets the value of the `"]
                    #[doc = #field_ident_str]
                    #[doc = "` field."]
                    #[inline(always)]
                    #vis fn #field_setter_ident (&mut self, value: [#elem; #len]) -> &mut Self{
                        const { Self::__assertions() };
                        for (i, elem) in value.into_iter().enumerate() {
                            self.#field_elem_setter_ident(i, elem);
                        }

                        self
                    }

                    #[doc = "Consumes `self` to modify the value of the `"]
                    #[doc = #field_ident_str]
                    #[doc = "` field and returns the modified `self`."]
                    #[inline(always)]
                    #vis fn #field_with_ident (mut self, value: [#elem; #len]) -> Self {
                        self.#field_setter_ident(value);
                        self
                    }
                })
            }
            FieldTy::Try(field_ty) => Ok(quote_spanned! {
                *span =>
                #[doc = "Sets the value of the `"]
                #[doc = #field_ident_str]
                #[doc = "` field."]
                #[inline(always)]
                #vis fn #field_setter_ident (&mut self, value: #field_ty) -> &mut Self {
                    #[allow(unused_imports)]
                    use bitos::{TryBits, BitUtils, integer::UnsignedInt};
                    const { Self::__assertions() };

                    let value_bits = value.to_bits();
                    let value_upcast = <#inner_ty as UnsignedInt>::new(
                        <<#field_ty as TryBits>::Bits as UnsignedInt>::value(value_bits)
                    );

                    self.0 = self.0.with_bits(#bits_start, #bits_end, value_upcast);
                    self
                }

                #[doc = "Consumes `self` to modify the value of the `"]
                #[doc = #field_ident_str]
                #[doc = "` field and returns the modified `self`."]
                #[inline(always)]
                #vis fn #field_with_ident (mut self, value: #field_ty) -> Self {
                    self.#field_setter_ident(value);
                    self
                }
            }),
        }
    }
}

struct BitStructInput {
    inner_ty: Box<Type>,
    bitos_attr: BitosAttr,
    phantom_data: Option<TokenStream>,
}

pub struct BitStruct {
    pub def: ItemStruct,
    pub impl_: ItemImpl,
    pub extra_impls: TokenStream,
}

impl BitStruct {
    pub fn new(bitos_attr: BitosAttr, mut s: ItemStruct) -> Result<Self, Error> {
        let inner_ty_name = format_ident!("u{}", bitos_attr.bitlen);
        let inner_ty =
            Box::new(parse_quote_spanned! { bitos_attr.span => ::bitos::integer::#inner_ty_name });

        let mut fields = Vec::new();
        let fields_err =
            s.fields
                .iter()
                .map(StructField::new)
                .fold(None, |acc: Option<Error>, r| match r {
                    Ok(f) => {
                        fields.push(f);
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

        if let Some(e) = fields_err {
            return Err(e);
        }

        let generics = &s.generics;
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        let ty_params = generics
            .params
            .iter()
            .filter(|p| matches!(p, syn::GenericParam::Type(_)))
            .collect::<Vec<_>>();

        let phantom_data = (!ty_params.is_empty())
            .then(|| quote::quote! { ::core::marker::PhantomData::<(#(#ty_params),*)> });

        let bitstruct = BitStructInput {
            inner_ty,
            bitos_attr,
            phantom_data,
        };

        let assertions = fields
            .iter()
            .map(|f| f.assertions(&bitstruct))
            .collect::<Vec<_>>();

        let masks = fields
            .iter()
            .map(|f| f.mask(&bitstruct))
            .collect::<Result<Vec<_>, _>>()?;

        let getters = fields
            .iter()
            .map(|f| f.getter(&bitstruct))
            .collect::<Result<Vec<_>, _>>()?;

        let setters = fields
            .iter()
            .map(|f| f.setters(&bitstruct))
            .collect::<Result<Vec<_>, _>>()?;

        let generate_debug = extract_derive("Debug", &mut s.attrs);

        let attrs = &s.attrs;
        let vis = &s.vis;
        let ident = &s.ident;
        let inner_ty = &bitstruct.inner_ty;
        let phantom_data = &bitstruct.phantom_data;

        let zerocopy = if cfg!(feature = "zerocopy") {
            Some(quote::quote! {
                #[derive(
                    ::zerocopy::KnownLayout,
                    ::zerocopy::Immutable,
                    ::zerocopy::IntoBytes,
                    ::zerocopy::FromBytes,
                )]
            })
        } else {
            None
        };

        let def = parse_quote_spanned! {
            bitstruct.bitos_attr.span =>
            #(#attrs)*
            #zerocopy
            #[repr(transparent)]
            #[allow(clippy::all)]
            #vis struct #ident #generics ( #inner_ty, #phantom_data );
        };

        let impl_ = parse_quote_spanned! {
            bitstruct.bitos_attr.span =>
            #[allow(dead_code, clippy::all)]
            impl #impl_generics #ident #ty_generics #where_clause {
                #(#masks)*

                #[doc(hidden)]
                const fn __assertions() {
                    #(#assertions)*
                }

                #[inline(always)]
                pub fn from_bits(value: <Self as ::bitos::TryBits>::Bits) -> Self {
                    const { Self::__assertions() };
                    Self(value, #phantom_data)
                }

                #[inline(always)]
                pub fn to_bits(&self) -> <Self as ::bitos::TryBits>::Bits {
                    const { Self::__assertions() };
                    self.0
                }

                #(#getters)*
                #(#setters)*
            }
        };

        let dbg = generate_debug.then(|| {
            let ty_ident_str = ident.to_string();
            let field_idents = fields.iter().map(|f| &f.ident);
            let field_idents_str = fields.iter().map(|f| f.ident.to_string());
            let mut generics = generics.clone();
            for param in generics.type_params_mut() {
                param.bounds.push(parse_quote! { ::core::fmt::Debug });
            }

            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

            quote::quote! {
                #[allow(clippy::all)]
                impl #impl_generics ::core::fmt::Debug for #ident #ty_generics #where_clause {
                    #[inline]
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        f.debug_struct(#ty_ident_str)
                            #(.field(#field_idents_str, &self.#field_idents()))*
                            .finish()
                    }
                }
            }
        });

        let extra_impls = quote::quote! {
            #dbg

            #[allow(clippy::all)]
            impl #impl_generics ::bitos::TryBits for #ident #ty_generics #where_clause {
                type Bits = #inner_ty;

                #[inline(always)]
                fn try_from_bits(value: Self::Bits) -> ::core::option::Option<Self> {
                    Some(Self(value, #phantom_data))
                }

                #[inline(always)]
                fn to_bits(&self) -> Self::Bits {
                    self.0
                }
            }

            #[allow(clippy::all)]
            impl #impl_generics ::bitos::Bits for #ident #ty_generics #where_clause {
                #[inline(always)]
                fn from_bits(value: Self::Bits) -> Self {
                    Self(value, #phantom_data)
                }
            }
        };

        Ok(BitStruct {
            def,
            impl_,
            extra_impls,
        })
    }
}

impl ToTokens for BitStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            def,
            impl_,
            extra_impls,
        } = self;

        tokens.extend(quote::quote! {
            #def
            #impl_
            #extra_impls
        });
    }
}
