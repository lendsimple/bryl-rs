//! `#[derive(Code)]`.

use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident, Lit};

use crate::attrs::Errors;

enum CodeLit {
    Str(String),
    Num(u64),
}

/// Whether codes are strings (alpha fields) or integers (numeric fields).
#[derive(Clone, Copy, PartialEq)]
enum Repr {
    Str,
    Num,
}

struct Variant<'a> {
    ident: &'a Ident,
    code: CodeLit,
    span: Span,
}

impl Variant<'_> {
    fn code_tokens(&self) -> TokenStream {
        match &self.code {
            CodeLit::Str(s) => quote! { #s },
            CodeLit::Num(n) => quote! { #n },
        }
    }
}

pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let variants = parse_variants(input)?;
    let repr = check_codes(&variants)?;
    let name = &input.ident;
    let inherent = inherent_impl(&variants, repr);
    let conversions = conversion_impls(name, repr);
    let field_value = field_value_impl(name, repr);
    Ok(quote! {
        #[automatically_derived]
        impl #name {
            #inherent
        }
        #conversions
        #field_value
    })
}

fn parse_variants(input: &DeriveInput) -> syn::Result<Vec<Variant<'_>>> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            input.generics.span(),
            "`#[derive(Code)]` does not support generic enums",
        ));
    }
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "`#[derive(Code)]` only supports enums",
        ));
    };
    if data.variants.is_empty() {
        return Err(syn::Error::new(
            input.ident.span(),
            "a code table needs at least one variant",
        ));
    }
    let mut errors = Errors::default();
    let mut variants = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            errors.push(syn::Error::new(
                variant.fields.span(),
                "code table variants cannot have fields",
            ));
            continue;
        }
        match parse_code(variant) {
            Ok((code, span)) => variants.push(Variant {
                ident: &variant.ident,
                code,
                span,
            }),
            Err(err) => errors.push(err),
        }
    }
    errors.finish()?;
    Ok(variants)
}

/// All codes must share one representation and be unique.
fn check_codes(variants: &[Variant<'_>]) -> syn::Result<Repr> {
    let repr_of = |v: &Variant<'_>| match v.code {
        CodeLit::Str(_) => Repr::Str,
        CodeLit::Num(_) => Repr::Num,
    };
    let repr = repr_of(&variants[0]);
    let mut errors = Errors::default();
    let mut seen: HashMap<String, &Ident> = HashMap::new();
    for variant in variants {
        if repr_of(variant) != repr {
            errors.push(syn::Error::new(
                variant.span,
                "all codes must be strings or all must be integers",
            ));
            continue;
        }
        let key = match &variant.code {
            CodeLit::Str(s) => s.clone(),
            CodeLit::Num(n) => n.to_string(),
        };
        if let Some(previous) = seen.insert(key.clone(), variant.ident) {
            errors.push(syn::Error::new(
                variant.span,
                format!("code {key:?} is already used by `{previous}`"),
            ));
        }
    }
    errors.finish()?;
    Ok(repr)
}

/// `ALL`, `as_code` and `from_code`.
fn inherent_impl(variants: &[Variant<'_>], repr: Repr) -> TokenStream {
    let idents: Vec<_> = variants.iter().map(|v| v.ident).collect();
    let codes: Vec<_> = variants.iter().map(Variant::code_tokens).collect();
    let (code_ty, from_code_sig) = match repr {
        Repr::Str => (
            quote! { &'static str },
            quote! { pub fn from_code(code: &str) },
        ),
        Repr::Num => (quote! { u64 }, quote! { pub const fn from_code(code: u64) }),
    };
    quote! {
        /// Every variant, in declaration order.
        pub const ALL: &'static [Self] = &[#(Self::#idents),*];

        /// The variant's code as written in a record.
        pub const fn as_code(&self) -> #code_ty {
            match self {
                #( Self::#idents => #codes, )*
            }
        }

        /// Looks up the variant for a code.
        #from_code_sig -> ::core::option::Option<Self> {
            match code {
                #( #codes => ::core::option::Option::Some(Self::#idents), )*
                _ => ::core::option::Option::None,
            }
        }
    }
}

/// `Display`, `FromStr` and `TryFrom`.
fn conversion_impls(name: &Ident, repr: Repr) -> TokenStream {
    let name_str = name.to_string();
    let unknown = |code: TokenStream| {
        quote! {
            ::bryl::UnknownCode {
                type_name: #name_str,
                code: ::std::string::ToString::to_string(#code),
            }
        }
    };
    let unknown_str = unknown(quote! { s });
    let (parse_str, try_from) = match repr {
        Repr::Str => (
            quote! { Self::from_code(s) },
            quote! {
                #[automatically_derived]
                impl ::core::convert::TryFrom<&str> for #name {
                    type Error = ::bryl::UnknownCode;

                    fn try_from(code: &str) -> ::core::result::Result<Self, Self::Error> {
                        ::core::str::FromStr::from_str(code)
                    }
                }
            },
        ),
        Repr::Num => {
            let unknown_num = unknown(quote! { &code });
            (
                quote! { s.parse::<u64>().ok().and_then(Self::from_code) },
                quote! {
                    #[automatically_derived]
                    impl ::core::convert::TryFrom<u64> for #name {
                        type Error = ::bryl::UnknownCode;

                        fn try_from(code: u64) -> ::core::result::Result<Self, Self::Error> {
                            Self::from_code(code).ok_or_else(|| #unknown_num)
                        }
                    }
                },
            )
        }
    };
    quote! {
        #[automatically_derived]
        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.as_code(), f)
            }
        }

        #[automatically_derived]
        impl ::core::str::FromStr for #name {
            type Err = ::bryl::UnknownCode;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                (#parse_str).ok_or_else(|| #unknown_str)
            }
        }

        #try_from
    }
}

/// `FieldValue` plus the kind marker trait. Codes are never sanitized.
fn field_value_impl(name: &Ident, repr: Repr) -> TokenStream {
    let name_str = name.to_string();
    let (kind_ok, marker, encode, decode) = match repr {
        Repr::Str => (
            quote! { spec.kind == ::bryl::FieldKind::Alpha },
            quote! { ::bryl::kind::AlphaField },
            quote! { ::bryl::codec::encode_alpha(spec, self.as_code(), out) },
            quote! {
                let value = ::bryl::codec::decode_alpha(spec, raw)?;
                Self::from_code(&value).ok_or(::bryl::FieldErrorKind::UnknownCode(value))
            },
        ),
        Repr::Num => (
            quote! { ::core::matches!(spec.kind, ::bryl::FieldKind::Numeric { .. }) },
            quote! { ::bryl::kind::NumericField },
            quote! { ::bryl::codec::encode_numeric(spec, self.as_code(), out) },
            quote! {
                let value = ::bryl::codec::decode_numeric(spec, raw)?;
                Self::from_code(value).ok_or_else(|| {
                    ::bryl::FieldErrorKind::UnknownCode(::std::string::ToString::to_string(&value))
                })
            },
        ),
    };
    let check_kind = quote! {
        if !(#kind_ok) {
            return ::core::result::Result::Err(::bryl::FieldErrorKind::KindMismatch {
                type_name: #name_str,
                kind: spec.kind.name(),
            });
        }
    };
    quote! {
        #[automatically_derived]
        impl ::bryl::FieldValue for #name {
            fn encode(
                &self,
                spec: &::bryl::FieldSpec,
                _sanitize: ::bryl::Sanitize,
                out: &mut ::std::vec::Vec<u8>,
            ) -> ::core::result::Result<(), ::bryl::FieldErrorKind> {
                #check_kind
                #encode
            }

            fn decode(
                spec: &::bryl::FieldSpec,
                raw: &[u8],
            ) -> ::core::result::Result<Self, ::bryl::FieldErrorKind> {
                #check_kind
                #decode
            }
        }

        #[automatically_derived]
        impl #marker for #name {}
    }
}

fn parse_code(variant: &syn::Variant) -> syn::Result<(CodeLit, Span)> {
    let mut found = None;
    for attr in variant.attrs.iter().filter(|a| a.path().is_ident("code")) {
        if found.is_some() {
            return Err(syn::Error::new(attr.span(), "duplicate `#[code(...)]`"));
        }
        let lit: Lit = attr.parse_args()?;
        let code = match &lit {
            Lit::Str(s) => CodeLit::Str(check_str_code(&s.value(), lit.span())?),
            Lit::Int(i) => CodeLit::Num(i.base10_parse()?),
            _ => {
                return Err(syn::Error::new(
                    lit.span(),
                    "expected a string or integer code",
                ));
            }
        };
        found = Some((code, lit.span()));
    }
    found.ok_or_else(|| {
        syn::Error::new(
            variant.ident.span(),
            "missing `#[code(\"...\")]` or `#[code(N)]`",
        )
    })
}

fn check_str_code(value: &str, span: Span) -> syn::Result<String> {
    if value.is_empty() {
        return Err(syn::Error::new(span, "codes cannot be empty"));
    }
    if !value.chars().all(|c| matches!(c, ' '..='~')) {
        return Err(syn::Error::new(span, "codes must be printable ASCII"));
    }
    if value.starts_with(' ') || value.ends_with(' ') {
        return Err(syn::Error::new(
            span,
            "codes cannot start or end with a space (padding is stripped on decode)",
        ));
    }
    Ok(value.to_owned())
}
