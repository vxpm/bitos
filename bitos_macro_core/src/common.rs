use proc_macro2::Span;
use quote::ToTokens;
use syn::{Attribute, Error, Expr, LitInt, parse::Parse, punctuated::Punctuated, spanned::Spanned};

pub fn extract_attr(ident: &str, attrs: &mut Vec<Attribute>) -> Option<Attribute> {
    let index = attrs
        .iter()
        .enumerate()
        .find_map(|(i, a)| a.meta.path().is_ident(ident).then_some(i))?;

    Some(attrs.remove(index))
}

pub fn extract_derive(ident: &str, attrs: &mut Vec<Attribute>) -> bool {
    let Some(attr) = attrs.iter_mut().find(|a| a.meta.path().is_ident("derive")) else {
        return false;
    };

    let syn::Meta::List(list) = &mut attr.meta else {
        return false;
    };

    let parser = syn::punctuated::Punctuated::<syn::Path, syn::token::Comma>::parse_terminated;
    let Ok(mut args) = list.parse_args_with(parser) else {
        return false;
    };

    let contains = args.iter().find(|a| a.is_ident(ident)).is_some();
    if contains {
        args = Punctuated::from_iter(args.into_iter().filter(|a| !a.is_ident(ident)));
        list.tokens = args.to_token_stream();
    }

    contains
}

pub struct BitosAttr {
    pub span: Span,
    pub bitlen: usize,
}

impl Parse for BitosAttr {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, Error> {
        let bitlen = input.parse::<LitInt>()?;
        let bitlen = bitlen.base10_parse::<usize>()?;

        Ok(Self {
            span: input.span(),
            bitlen,
        })
    }
}

pub enum Bitrange {
    HalfOpen { start: usize, end: Option<usize> },
    Closed { start: usize, end: Option<usize> },
}

impl Bitrange {
    pub fn start(&self) -> usize {
        match self {
            Bitrange::HalfOpen { start, .. } | Bitrange::Closed { start, .. } => *start,
        }
    }

    pub fn end(&self) -> Option<usize> {
        match self {
            Bitrange::HalfOpen { end, .. } | Bitrange::Closed { end, .. } => *end,
        }
    }
}

pub struct BitsAttr {
    pub span: Span,
    pub bitrange: Bitrange,
}

impl BitsAttr {
    pub fn extract(attrs: &mut Vec<Attribute>) -> Result<Option<Self>, Error> {
        let expect_lit_int = |e: Box<Expr>| {
            if let syn::Expr::Lit(lit_expr) = &*e
                && let syn::Lit::Int(int_lit) = &lit_expr.lit
            {
                int_lit.base10_parse()
            } else {
                Err(Error::new(e.span(), "must be an integer literal"))
            }
        };

        let Some(bitos_attr) = extract_attr("bits", attrs) else {
            return Ok(None);
        };

        let bitrange = if let Ok(int_lit) = bitos_attr.parse_args::<LitInt>() {
            let start = int_lit.base10_parse()?;
            Bitrange::HalfOpen {
                start,
                end: Some(start + 1),
            }
        } else {
            let range_expr = bitos_attr.parse_args::<syn::ExprRange>()?;
            match range_expr.limits {
                syn::RangeLimits::HalfOpen(_) => {
                    let start = range_expr.start.map(expect_lit_int).unwrap_or(Ok(0))?;
                    let end = range_expr.end.map(expect_lit_int).transpose()?;

                    Bitrange::HalfOpen { start, end }
                }
                syn::RangeLimits::Closed(_) => {
                    let start = range_expr.start.map(expect_lit_int).unwrap_or(Ok(0))?;
                    let end = range_expr
                        .end
                        .map(expect_lit_int)
                        .transpose()?
                        .map(|x| x + 1);

                    Bitrange::Closed { start, end }
                }
            }
        };

        Ok(Some(Self {
            span: bitos_attr.span(),
            bitrange,
        }))
    }
}
