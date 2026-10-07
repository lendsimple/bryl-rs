//! `#[derive(Record)]`.

use bryl_pattern::Token;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident, Type};

use crate::attrs::{
    Align, ConstLit, Errors, FieldAttrs, Kind, RecordAttrs, SanitizeFlags, ValueAttrs,
    parse_field_attrs, parse_record_attrs,
};

struct Field<'a> {
    ident: &'a Ident,
    ty: &'a Type,
    attrs: FieldAttrs,
}

pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let (record_attrs, fields) = parse_input(input)?;
    let name = &input.ident;
    let name_str = name.to_string();

    let mut layout = Layout::default();
    for (index, field) in fields.iter().enumerate() {
        layout.push(index, field);
    }
    let Layout {
        literal_offset,
        flattened,
        spec_consts,
        constant_accessors,
        mut assertions,
        fields_entries,
        encode_steps,
        decode_inits,
    } = layout;

    let length_expr = offset_expr(literal_offset, &flattened);
    let fields_expr = fields_expr(name, &fields_entries);
    if let Some((declared, span)) = record_attrs.length {
        if flattened.is_empty() {
            if declared != literal_offset {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "declared length {declared} does not match the sum of the field lengths, {literal_offset}"
                    ),
                ));
            }
        } else {
            // Flattened lengths are only known to the compiler.
            let message = format!(
                "{name_str}: declared length {declared} does not match the sum of the field lengths"
            );
            assertions.push(quote_spanned! {span=>
                const _: () = ::core::assert!(<#name as ::bryl::Record>::LENGTH == #declared, #message);
            });
        }
    }

    let sanitize_const = record_attrs.sanitize.map(|flags| {
        let sanitize = sanitize_expr(flags);
        quote! { const SANITIZE: ::bryl::Sanitize = #sanitize; }
    });

    Ok(quote! {
        #[automatically_derived]
        impl #name {
            #(#spec_consts)*
            #(#constant_accessors)*
        }

        #[automatically_derived]
        impl ::bryl::Record for #name {
            const NAME: &'static str = #name_str;
            const LENGTH: usize = #length_expr;
            const FIELDS: &'static [::bryl::FieldSpec] = #fields_expr;
            #sanitize_const

            fn encode_into(
                &self,
                cx: &::bryl::EncodeCx,
                out: &mut ::std::vec::Vec<u8>,
            ) -> ::core::result::Result<(), ::bryl::Error> {
                #(#encode_steps)*
                ::core::result::Result::Ok(())
            }

            fn decode_fields(raw: &[u8]) -> ::core::result::Result<Self, ::bryl::Error> {
                ::core::result::Result::Ok(Self {
                    #(#decode_inits,)*
                })
            }
        }

        #(#assertions)*
    })
}

fn parse_input(input: &DeriveInput) -> syn::Result<(RecordAttrs, Vec<Field<'_>>)> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            input.generics.span(),
            "`#[derive(Record)]` does not support generic structs",
        ));
    }
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.ident.span(),
            "`#[derive(Record)]` only supports structs",
        ));
    };
    let Fields::Named(named) = &data.fields else {
        return Err(syn::Error::new(
            data.fields.span(),
            "`#[derive(Record)]` needs a struct with named fields",
        ));
    };
    if named.named.is_empty() {
        return Err(syn::Error::new(
            input.ident.span(),
            "a record needs at least one field",
        ));
    }

    let mut errors = Errors::default();
    let record_attrs = parse_record_attrs(&input.attrs)
        .map_err(|err| errors.push(err))
        .ok();
    let mut fields = Vec::new();
    for field in &named.named {
        let ident = field.ident.as_ref().expect("named field");
        match parse_field_attrs(&field.attrs, &field.ty, ident.span()) {
            Ok(attrs) => fields.push(Field {
                ident,
                ty: &field.ty,
                attrs,
            }),
            Err(err) => errors.push(err),
        }
    }
    errors.finish()?;
    Ok((record_attrs.expect("no errors"), fields))
}

/// `literal + <Flattened>::LENGTH + ...`
fn offset_expr(literal: usize, flattened: &[&Type]) -> TokenStream {
    quote! { #literal #( + <#flattened as ::bryl::Record>::LENGTH )* }
}

/// Generated pieces, accumulated field by field.
#[derive(Default)]
struct Layout<'a> {
    /// Sum of the non-flattened field lengths so far.
    literal_offset: usize,
    flattened: Vec<&'a Type>,
    spec_consts: Vec<TokenStream>,
    constant_accessors: Vec<TokenStream>,
    assertions: Vec<TokenStream>,
    fields_entries: Vec<FieldsEntry<'a>>,
    encode_steps: Vec<TokenStream>,
    decode_inits: Vec<TokenStream>,
}

impl<'a> Layout<'a> {
    fn push(&mut self, index: usize, field: &Field<'a>) {
        let ident = field.ident;
        let ty = field.ty;
        let offset = offset_expr(self.literal_offset, &self.flattened);
        match &field.attrs {
            FieldAttrs::Flatten => {
                self.assertions.push(quote_spanned! {ty.span()=>
                    const _: () = ::bryl::__private::assert_record::<#ty>();
                });
                self.encode_steps.push(quote! {
                    ::bryl::codec::encode_flattened::<Self, #ty>(#offset, &self.#ident, cx, out)?;
                });
                self.decode_inits.push(quote! {
                    #ident: ::bryl::codec::decode_flattened::<Self, #ty>(#offset, raw)?
                });
                self.fields_entries.push(FieldsEntry::Flatten(ty, offset));
                self.flattened.push(ty);
            }
            FieldAttrs::Value(attrs) => {
                let const_name = format_ident!("__BRYL_FIELD_{}", index);
                let spec = spec_expr(&ident_name(ident), &offset, attrs);
                self.spec_consts.push(quote! {
                    #[doc(hidden)]
                    const #const_name: ::bryl::FieldSpec = #spec;
                });
                self.assertions.push(kind_assertion(ty, &attrs.kind));
                self.constant_accessors
                    .extend(constant_accessor(ident, attrs));
                self.encode_steps.push(quote! {
                    ::bryl::codec::encode_field::<Self, _>(&Self::#const_name, &self.#ident, cx, out)?;
                });
                self.decode_inits.push(quote! {
                    #ident: ::bryl::codec::decode_field::<Self, _>(&Self::#const_name, raw)?
                });
                self.fields_entries.push(FieldsEntry::Single(const_name));
                self.literal_offset += attrs.length;
            }
        }
    }
}

enum FieldsEntry<'a> {
    Single(Ident),
    Flatten(&'a Type, TokenStream),
}

/// `FIELDS`: a plain slice, or, with flattened records, a slice assembled in a
/// const block from the embedded records' (shifted) fields.
fn fields_expr(name: &Ident, entries: &[FieldsEntry<'_>]) -> TokenStream {
    if entries.iter().all(|e| matches!(e, FieldsEntry::Single(_))) {
        let consts = entries.iter().map(|e| match e {
            FieldsEntry::Single(c) => c,
            FieldsEntry::Flatten(..) => unreachable!(),
        });
        return quote! { &[#(Self::#consts),*] };
    }
    let counts = entries.iter().map(|e| match e {
        FieldsEntry::Single(_) => quote! { 1 },
        FieldsEntry::Flatten(ty, _) => quote! { <#ty as ::bryl::Record>::FIELDS.len() },
    });
    let fills = entries.iter().map(|e| match e {
        FieldsEntry::Single(c) => quote! {
            fields[n] = #name::#c;
            n += 1;
        },
        FieldsEntry::Flatten(ty, offset) => quote! {
            let inner = <#ty as ::bryl::Record>::FIELDS;
            let mut i = 0;
            while i < inner.len() {
                fields[n] = inner[i].shifted(#offset);
                n += 1;
                i += 1;
            }
        },
    });
    quote! {{
        const COUNT: usize = 0 #( + #counts )*;
        const FIELDS: [::bryl::FieldSpec; COUNT] = {
            let mut fields = [::bryl::FieldSpec::alpha("", 0, 0); COUNT];
            let mut n = 0;
            #(#fills)*
            fields
        };
        &FIELDS
    }}
}

fn ident_name(ident: &Ident) -> String {
    let name = ident.to_string();
    name.strip_prefix("r#").map_or(name.clone(), str::to_owned)
}

fn spec_expr(name: &str, offset: &TokenStream, attrs: &ValueAttrs) -> TokenStream {
    let length = attrs.length;
    let mut spec = match &attrs.kind {
        Kind::Alpha => quote! { ::bryl::FieldSpec::alpha(#name, #offset, #length) },
        Kind::Numeric => quote! { ::bryl::FieldSpec::numeric(#name, #offset, #length) },
        Kind::Date(tokens) => {
            let tokens = tokens.iter().copied().map(token_expr);
            quote! { ::bryl::FieldSpec::date(#name, #offset, &[#(#tokens),*]) }
        }
        Kind::Time(tokens) => {
            let tokens = tokens.iter().copied().map(token_expr);
            quote! { ::bryl::FieldSpec::time(#name, #offset, &[#(#tokens),*]) }
        }
        Kind::DateTime(tokens) => {
            let tokens = tokens.iter().copied().map(token_expr);
            quote! { ::bryl::FieldSpec::datetime(#name, #offset, &[#(#tokens),*]) }
        }
    };
    if let Some(pad) = attrs.pad {
        spec = quote! { #spec.with_pad(#pad) };
    }
    if let Some(align) = attrs.align {
        let align = match align {
            Align::Left => quote! { ::bryl::Align::Left },
            Align::Right => quote! { ::bryl::Align::Right },
        };
        spec = quote! { #spec.with_align(#align) };
    }
    if let Some(min) = attrs.min {
        spec = quote! { #spec.with_min(#min) };
    }
    if let Some(max) = attrs.max {
        spec = quote! { #spec.with_max(#max) };
    }
    match &attrs.constant {
        Some(ConstLit::Str(value)) => spec = quote! { #spec.with_constant_str(#value) },
        Some(ConstLit::Num(value)) => spec = quote! { #spec.with_constant_num(#value) },
        None => {}
    }
    if attrs.reserved {
        spec = quote! { #spec.reserved() };
    }
    if let Some(flags) = attrs.sanitize {
        let sanitize = sanitize_expr(flags);
        spec = quote! { #spec.with_sanitize(#sanitize) };
    }
    spec
}

fn token_expr(token: Token) -> TokenStream {
    match token {
        Token::Year4 => quote! { ::bryl::Token::Year4 },
        Token::Year2 => quote! { ::bryl::Token::Year2 },
        Token::Month => quote! { ::bryl::Token::Month },
        Token::Day => quote! { ::bryl::Token::Day },
        Token::DayOfYear => quote! { ::bryl::Token::DayOfYear },
        Token::Hour24 => quote! { ::bryl::Token::Hour24 },
        Token::Hour12 => quote! { ::bryl::Token::Hour12 },
        Token::Minute => quote! { ::bryl::Token::Minute },
        Token::Second => quote! { ::bryl::Token::Second },
        Token::AmPm => quote! { ::bryl::Token::AmPm },
        Token::Literal(byte) => quote! { ::bryl::Token::Literal(#byte) },
    }
}

fn sanitize_expr(flags: SanitizeFlags) -> TokenStream {
    let SanitizeFlags {
        upper,
        filter,
        truncate,
    } = flags;
    quote! { ::bryl::Sanitize::NONE.upper(#upper).filter(#filter).truncate(#truncate) }
}

/// Requires the field type to implement the kind's marker trait, so a type
/// that cannot be stored in the field is a compile error at the field.
fn kind_assertion(ty: &Type, kind: &Kind) -> TokenStream {
    let assert = match kind {
        Kind::Alpha => quote! { assert_alpha },
        Kind::Numeric => quote! { assert_numeric },
        Kind::Date(_) => quote! { assert_date },
        Kind::Time(_) => quote! { assert_time },
        Kind::DateTime(_) => quote! { assert_datetime },
    };
    quote_spanned! {ty.span()=>
        const _: () = ::bryl::__private::#assert::<#ty>();
    }
}

/// `pub const SEGMENT_IDENTIFIER: &str = "K1";` for each constant field.
fn constant_accessor(ident: &Ident, attrs: &ValueAttrs) -> Option<TokenStream> {
    let name = format_ident!("{}", ident_name(ident).to_uppercase());
    let doc = format!("Constant value of the `{}` field.", ident_name(ident));
    match attrs.constant.as_ref()? {
        ConstLit::Str(value) => Some(quote! {
            #[doc = #doc]
            pub const #name: &'static str = #value;
        }),
        ConstLit::Num(value) => Some(quote! {
            #[doc = #doc]
            pub const #name: u64 = #value;
        }),
    }
}
