//! Parsing and validation of `#[bryl(...)]` attributes.

use bryl_pattern::{PatternKind, Token, parse_pattern, pattern_width};
use proc_macro2::Span;
use syn::meta::ParseNestedMeta;
use syn::spanned::Spanned;
use syn::{Attribute, Lit, LitInt, LitStr, Type};

/// Struct-level `#[bryl(...)]`.
#[derive(Default)]
pub struct RecordAttrs {
    pub sanitize: Option<SanitizeFlags>,
    pub length: Option<(usize, Span)>,
}

/// `sanitize(upper, filter, truncate)`; `no_sanitize` is all false.
#[derive(Default, Clone, Copy)]
pub struct SanitizeFlags {
    pub upper: bool,
    pub filter: bool,
    pub truncate: bool,
}

pub enum Kind {
    Alpha,
    Numeric,
    Date(Vec<Token>),
    Time(Vec<Token>),
    DateTime(Vec<Token>),
}

impl Kind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Alpha => "alpha",
            Self::Numeric => "numeric",
            Self::Date(_) => "date",
            Self::Time(_) => "time",
            Self::DateTime(_) => "datetime",
        }
    }
}

pub enum ConstLit {
    Str(String),
    Num(u64),
}

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Right,
}

/// A field's `#[bryl(...)]`, validated against its type.
pub enum FieldAttrs {
    Flatten,
    Value(ValueAttrs),
}

pub struct ValueAttrs {
    pub kind: Kind,
    pub length: usize,
    pub pad: Option<u8>,
    pub align: Option<Align>,
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub constant: Option<ConstLit>,
    pub reserved: bool,
    pub sanitize: Option<SanitizeFlags>,
}

/// Collects errors so several mistakes are reported at once.
#[derive(Default)]
pub struct Errors(Option<syn::Error>);

impl Errors {
    pub fn push(&mut self, err: syn::Error) {
        match &mut self.0 {
            Some(existing) => existing.combine(err),
            None => self.0 = Some(err),
        }
    }

    pub fn finish(self) -> syn::Result<()> {
        self.0.map_or(Ok(()), Err)
    }
}

fn bryl_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs.iter().filter(|a| a.path().is_ident("bryl"))
}

pub fn parse_record_attrs(attrs: &[Attribute]) -> syn::Result<RecordAttrs> {
    let mut out = RecordAttrs::default();
    for attr in bryl_attrs(attrs) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("sanitize") {
                out.sanitize = Some(parse_sanitize(&meta)?);
            } else if meta.path.is_ident("length") {
                let lit: LitInt = meta.value()?.parse()?;
                out.length = Some((lit.base10_parse()?, lit.span()));
            } else {
                return Err(meta
                    .error("unknown record attribute; expected `sanitize(...)` or `length = N`"));
            }
            Ok(())
        })?;
    }
    Ok(out)
}

fn parse_sanitize(meta: &ParseNestedMeta) -> syn::Result<SanitizeFlags> {
    let mut flags = SanitizeFlags::default();
    meta.parse_nested_meta(|flag| {
        if flag.path.is_ident("upper") {
            flags.upper = true;
        } else if flag.path.is_ident("filter") {
            flags.filter = true;
        } else if flag.path.is_ident("truncate") {
            flags.truncate = true;
        } else {
            return Err(flag.error("expected `upper`, `filter` or `truncate`"));
        }
        Ok(())
    })?;
    Ok(flags)
}

/// Raw options before validation.
#[derive(Default)]
struct RawField {
    kind: Option<(Kind, usize, Span)>,
    flatten: Option<Span>,
    pad: Option<(u8, Span)>,
    align: Option<Align>,
    min: Option<(u64, Span)>,
    max: Option<(u64, Span)>,
    constant: Option<(ConstLit, Span)>,
    reserved: Option<Span>,
    sanitize: Option<SanitizeFlags>,
}

pub fn parse_field_attrs(
    attrs: &[Attribute],
    ty: &Type,
    field_span: Span,
) -> syn::Result<FieldAttrs> {
    let mut raw = RawField::default();
    for attr in bryl_attrs(attrs) {
        attr.parse_nested_meta(|meta| raw.parse_option(&meta))?;
    }
    validate(raw, ty, field_span)
}

impl RawField {
    /// Parses one option of a field's `#[bryl(...)]`.
    fn parse_option(&mut self, meta: &ParseNestedMeta) -> syn::Result<()> {
        let span = meta.path.span();
        let ident = meta
            .path
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        match ident.as_str() {
            "alpha" => self.set_kind(Kind::Alpha, parse_length(meta)?, span),
            "numeric" => self.set_kind(Kind::Numeric, parse_length(meta)?, span),
            "date" => self.set_pattern(meta, PatternKind::Date, span),
            "time" => self.set_pattern(meta, PatternKind::Time, span),
            "datetime" => self.set_pattern(meta, PatternKind::DateTime, span),
            "flatten" => {
                if self.kind.is_some() {
                    return Err(syn::Error::new(span, "a field can have only one kind"));
                }
                self.flatten = Some(span);
                Ok(())
            }
            "pad" => {
                self.pad = Some(parse_pad(meta)?);
                Ok(())
            }
            "align" => {
                self.align = Some(parse_align(meta)?);
                Ok(())
            }
            "min" => {
                self.min = Some(parse_bound(meta)?);
                Ok(())
            }
            "max" => {
                self.max = Some(parse_bound(meta)?);
                Ok(())
            }
            "constant" => {
                self.constant = Some(parse_constant(meta)?);
                Ok(())
            }
            "reserved" => {
                self.reserved = Some(span);
                Ok(())
            }
            "sanitize" => {
                self.sanitize = Some(parse_sanitize(meta)?);
                Ok(())
            }
            "no_sanitize" => {
                self.sanitize = Some(SanitizeFlags::default());
                Ok(())
            }
            _ => Err(meta.error(
                "unknown field attribute; expected one of `alpha(N)`, `numeric(N)`, \
                 `date(\"...\")`, `time(\"...\")`, `datetime(\"...\")`, `flatten`, `pad`, \
                 `align`, `min`, `max`, `constant`, `reserved`, `sanitize(...)`, `no_sanitize`",
            )),
        }
    }

    fn set_kind(&mut self, kind: Kind, length: usize, span: Span) -> syn::Result<()> {
        if self.kind.is_some() || self.flatten.is_some() {
            return Err(syn::Error::new(span, "a field can have only one kind"));
        }
        self.kind = Some((kind, length, span));
        Ok(())
    }

    /// `date("...")`, `time("...")` or `datetime("...")`.
    fn set_pattern(
        &mut self,
        meta: &ParseNestedMeta,
        pattern_kind: PatternKind,
        span: Span,
    ) -> syn::Result<()> {
        let content;
        syn::parenthesized!(content in meta.input);
        let lit: LitStr = content.parse()?;
        let tokens = parse_pattern(&lit.value(), pattern_kind)
            .map_err(|err| syn::Error::new(lit.span(), err))?;
        let length = pattern_width(&tokens);
        let kind = match pattern_kind {
            PatternKind::Date => Kind::Date(tokens),
            PatternKind::Time => Kind::Time(tokens),
            PatternKind::DateTime => Kind::DateTime(tokens),
        };
        self.set_kind(kind, length, span)
    }
}

/// `(N)` after `alpha` or `numeric`.
fn parse_length(meta: &ParseNestedMeta) -> syn::Result<usize> {
    let content;
    syn::parenthesized!(content in meta.input);
    let lit: LitInt = content.parse()?;
    let length = lit.base10_parse()?;
    if length == 0 {
        return Err(syn::Error::new(
            lit.span(),
            "field length must be at least 1",
        ));
    }
    Ok(length)
}

/// `pad = ' '` or `pad = b' '`.
fn parse_pad(meta: &ParseNestedMeta) -> syn::Result<(u8, Span)> {
    let lit: Lit = meta.value()?.parse()?;
    let byte = match &lit {
        Lit::Char(c) => u8::try_from(c.value()).ok(),
        Lit::Byte(b) => Some(b.value()),
        _ => None,
    };
    match byte {
        Some(byte) if (b' '..=b'~').contains(&byte) => Ok((byte, lit.span())),
        _ => Err(syn::Error::new(
            lit.span(),
            "expected a printable ASCII character such as `' '`",
        )),
    }
}

/// `align = "left"` or `align = "right"`.
fn parse_align(meta: &ParseNestedMeta) -> syn::Result<Align> {
    let lit: LitStr = meta.value()?.parse()?;
    match lit.value().as_str() {
        "left" => Ok(Align::Left),
        "right" => Ok(Align::Right),
        _ => Err(syn::Error::new(
            lit.span(),
            "expected \"left\" or \"right\"",
        )),
    }
}

/// `min = N` or `max = N`.
fn parse_bound(meta: &ParseNestedMeta) -> syn::Result<(u64, Span)> {
    let lit: LitInt = meta.value()?.parse()?;
    Ok((lit.base10_parse()?, lit.span()))
}

/// `constant = "K1"` or `constant = 426`.
fn parse_constant(meta: &ParseNestedMeta) -> syn::Result<(ConstLit, Span)> {
    let lit: Lit = meta.value()?.parse()?;
    let value = match &lit {
        Lit::Str(s) => ConstLit::Str(s.value()),
        Lit::Int(i) => ConstLit::Num(i.base10_parse()?),
        _ => {
            return Err(syn::Error::new(
                lit.span(),
                "expected a string or integer constant",
            ));
        }
    };
    Ok((value, lit.span()))
}

fn validate(raw: RawField, ty: &Type, field_span: Span) -> syn::Result<FieldAttrs> {
    let mut errors = Errors::default();
    if let Some(span) = raw.flatten {
        let extras = [
            raw.pad.map(|p| p.1),
            raw.min.map(|m| m.1),
            raw.max.map(|m| m.1),
            raw.constant.as_ref().map(|c| c.1),
            raw.reserved,
        ];
        if extras.iter().any(Option::is_some) || raw.align.is_some() || raw.sanitize.is_some() {
            errors.push(syn::Error::new(
                span,
                "`flatten` cannot be combined with other options",
            ));
        }
        errors.finish()?;
        return Ok(FieldAttrs::Flatten);
    }
    let Some((kind, length, kind_span)) = raw.kind else {
        return Err(syn::Error::new(
            field_span,
            "missing field kind: add `#[bryl(alpha(N))]`, `numeric(N)`, `date(\"...\")`, \
             `time(\"...\")`, `datetime(\"...\")` or `flatten`",
        ));
    };
    let is_numeric = matches!(kind, Kind::Numeric);
    let is_alpha = matches!(kind, Kind::Alpha);

    for (bound, name) in [(raw.min, "min"), (raw.max, "max")] {
        if let Some((_, span)) = bound {
            if !is_numeric {
                errors.push(syn::Error::new(
                    span,
                    format!("`{name}` only applies to numeric fields"),
                ));
            }
        }
    }
    if raw.sanitize.is_some() && !is_alpha {
        errors.push(syn::Error::new(
            kind_span,
            "sanitize settings only apply to alpha fields",
        ));
    }
    if let (Some((_, c)), Some(r)) = (&raw.constant, raw.reserved) {
        let mut err = syn::Error::new(r, "a field cannot be both `constant` and `reserved`");
        err.combine(syn::Error::new(*c, "constant declared here"));
        errors.push(err);
    }
    if let Some(span) = raw.reserved {
        if !is_alpha && !is_numeric {
            errors.push(syn::Error::new(
                span,
                format!(
                    "{} fields have no filler value and cannot be `reserved`",
                    kind.name()
                ),
            ));
        }
    }
    if let Some((constant, span)) = &raw.constant {
        if let Err(msg) = check_constant(&kind, length, constant) {
            errors.push(syn::Error::new(*span, msg));
        }
    }
    let is_fixed = raw.constant.is_some() || raw.reserved.is_some();
    match (is_fixed, is_const_type(ty)) {
        (true, false) => errors.push(syn::Error::new(
            ty.span(),
            "constant and reserved fields must have type `bryl::Const`",
        )),
        (false, true) => errors.push(syn::Error::new(
            ty.span(),
            "`bryl::Const` fields need `constant = ...` or `reserved`",
        )),
        _ => {}
    }
    errors.finish()?;
    Ok(FieldAttrs::Value(ValueAttrs {
        kind,
        length,
        pad: raw.pad.map(|p| p.0),
        align: raw.align,
        min: raw.min.map(|m| m.0),
        max: raw.max.map(|m| m.0),
        constant: raw.constant.map(|c| c.0),
        reserved: raw.reserved.is_some(),
        sanitize: raw.sanitize,
    }))
}

fn check_constant(kind: &Kind, length: usize, constant: &ConstLit) -> Result<(), String> {
    match (kind, constant) {
        (Kind::Alpha, ConstLit::Str(value)) => {
            if !value.chars().all(|c| matches!(c, ' '..='~')) {
                return Err("constant must be printable ASCII".into());
            }
            if value.len() > length {
                return Err(format!(
                    "constant {value:?} is {} characters, longer than the field length {length}",
                    value.len()
                ));
            }
            Ok(())
        }
        (Kind::Numeric, ConstLit::Num(value)) => {
            let digits = value.to_string().len();
            if digits > length {
                return Err(format!(
                    "constant {value} has {digits} digits, more than the field length {length}"
                ));
            }
            Ok(())
        }
        (Kind::Alpha, ConstLit::Num(_)) => Err("alpha constants must be strings".into()),
        (Kind::Numeric, ConstLit::Str(_)) => Err("numeric constants must be integers".into()),
        (other, _) => Err(format!("{} fields cannot be constant", other.name())),
    }
}

/// True if the type is spelled `Const` or `bryl::Const` (any path ending in `Const`).
fn is_const_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path) if path.qself.is_none() => path
            .path
            .segments
            .last()
            .is_some_and(|seg| seg.ident == "Const" && seg.arguments.is_none()),
        _ => false,
    }
}
