use crate::internals::symbol::*;
use crate::internals::Ctxt;
use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote_spanned, quote};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::str::FromStr;
use syn::{self, parse_quote};
use syn::parse::{self, Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitInt, IntSuffix};
use syn::Meta::{List, NameValue, Word};
use syn::Lit::{Int, Float};
use syn::NestedMeta::{Literal, Meta};
use syn::spanned::Spanned;

pub struct Attr<'c, T> {
    cx: &'c Ctxt,
    name: Symbol,
    tokens: TokenStream,
    value: Option<T>,
}

impl<'c, T> Attr<'c, T> {
    fn none(cx: &'c Ctxt, name: Symbol) -> Self {
        Attr {
            cx: cx,
            name: name,
            tokens: TokenStream::new(),
            value: None,
        }
    }

    fn set<A: ToTokens>(&mut self, obj: A, value: T) {
        let tokens = obj.into_token_stream();

        if self.value.is_some() {
            self.cx.error_spanned_by(tokens, format!("duplicate lain attribute `{}`", self.name))
        } else {
            self.tokens = tokens;
            self.value = Some(value);
        }
    }

    fn set_opt<A: ToTokens>(&mut self, obj: A, value: Option<T>) {
        if let Some(value) = value {
            self.set(obj, value);
        }
    }

    fn set_if_none(&mut self, value: T) {
        if self.value.is_none() {
            self.value = Some(value);
        }
    }

    fn get(self) -> Option<T> {
        self.value
    }

    fn get_with_tokens(self) -> Option<(TokenStream, T)> {
        match self.value {
            Some(v) => Some((self.tokens, v)),
            None => None,
        }
    }
}

struct BoolAttr<'c>(Attr<'c, ()>);

impl<'c> BoolAttr<'c> {
    fn none(cx: &'c Ctxt, name: Symbol) -> Self {
        BoolAttr(Attr::none(cx, name))
    }

    fn set_true<A: ToTokens>(&mut self, obj: A) {
        self.0.set(obj, ());
    }

    fn get(&self) -> bool {
        self.0.value.is_some()
    }
}

/// Represents a struct or enum attribute information
pub struct Container {
    serialized_size: Option<usize>,
}

impl Container {
    /// Extract out the `#[lain()]` attributes from an item
    pub fn from_ast(cx: &Ctxt, item: &syn::DeriveInput) -> Self {
        let mut serialized_size = Attr::none(cx, SERIALIZED_SIZE);

        for meta_items in item.attrs.iter().filter_map(get_lain_meta_items) {
            for meta_item in meta_items {
                match meta_item {
                    Meta(NameValue(ref m)) if m.ident == SERIALIZED_SIZE => {
                        if let Int(ref i) = m.lit {
                            serialized_size.set(&m.ident, i.value() as usize);
                        } else {
                            cx.error_spanned_by(&m.lit, format!("failed to integer expression for {}", SERIALIZED_SIZE));
                        }
                    }
                    Meta(ref meta_item) => {
                        cx.error_spanned_by(
                            meta_item.name(),
                            format!("unknown lain container attribute `{}`", meta_item.name()),
                        );
                    }
                    Literal(ref lit) => {
                        cx.error_spanned_by(lit, "unexpected literal in lain container attribute");
                    }
                }
            }
        }

        Container {
            serialized_size: serialized_size.get(),
        }
    }

    pub fn serialized_size(&self) -> Option<usize> {
        self.serialized_size.clone()
    }

    pub fn lain_path(&self) -> Cow<syn::Path> {
        Cow::Owned(parse_quote!(_lain))
    }
}

#[derive(PartialEq)]
pub enum WeightTo {
    None,
    Min,
    Max,
}

impl ToTokens for WeightTo {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            WeightTo::None => tokens.extend(quote! {_lain::types::Weighted::None}),
            WeightTo::Min => tokens.extend(quote! {_lain::types::Weighted::Min}),
            WeightTo::Max => tokens.extend(quote! {_lain::types::Weighed::Max}),
        }
    }
}


pub fn unraw(ident: &Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

/// Represents field attribute information
pub struct Field {
    name: String,
    bits: Option<usize>,
    bit_shift: Option<usize>,
    bitfield_type: Option<syn::Type>,
    min: Option<TokenStream>,
    max: Option<TokenStream>,
    ignore: bool,
    ignore_chance: Option<f64>,
    initializer: Option<TokenStream>,
    little_endian: bool,
    big_endian: bool,
    weight_to: Option<WeightTo>
}

impl Field {
    /// Extract out the `#[lain()]` attributes from an item
    pub fn from_ast(cx: &Ctxt, index: usize, field: &syn::Field) -> Self {
        let mut bits = Attr::none(cx, BITS);
        let mut bitfield_type = Attr::none(cx, BITFIELD_TYPE);
        let mut min = Attr::none(cx, MIN);
        let mut max= Attr::none(cx, MAX);
        let mut ignore = BoolAttr::none(cx, IGNORE);
        let mut ignore_chance = Attr::none(cx, IGNORE_CHANCE);
        let mut initializer = Attr::none(cx, INITIALIZER);
        let mut big_endian= BoolAttr::none(cx, BIG_ENDIAN);
        let mut little_endian = BoolAttr::none(cx, LITTLE_ENDIAN);
        let mut weight_to = Attr::none(cx, WEIGHT_TO);

        let ident = match field.ident {
            Some(ref ident) => unraw(ident),
            None => index.to_string(),
        };

        for meta_items in field.attrs.iter().filter_map(get_lain_meta_items) {
            for meta_item in meta_items {
                match meta_item {
                    // `#[lain(min = 3)]`
                    Meta(NameValue(ref m)) if m.ident == MIN => {
                        if let Ok(t) = parse_min_max(cx, MIN, &m.lit) {
                            min.set(&m.ident, t);
                        }
                    }
                    // `#[lain(max = 3)]`
                    Meta(NameValue(ref m)) if m.ident == MAX => {
                        if let Ok(t) = parse_min_max(cx, MAX, &m.lit) {
                            max.set(&m.ident, t);
                        }
                    }
                    // `#[lain(bits = 3)]`
                    Meta(NameValue(ref m)) if m.ident == BITS => {
                        if let Int(ref i) = m.lit {
                            bits.set(&m.ident, i.value() as usize);
                        } else {
                            cx.error_spanned_by(&m.lit, format!("failed to parse integer expression for `{}`", BITS));
                        }
                    }
                    // `#[lain(bitfield_type = "u8")]`
                    Meta(NameValue(ref m)) if m.ident == BITFIELD_TYPE => {
                        if let Ok(expr) = parse_lit_into_type(&cx, BITFIELD_TYPE, &m.lit) {
                            bitfield_type.set(&m.ident, expr)
                        } else {
                            cx.error_spanned_by(&m.lit, format!("failed to parse `{}` into a type", BITFIELD_TYPE));
                        }
                    }
                    // `#[lain(big_endian)]`
                    Meta(Word(ref word)) if word == BIG_ENDIAN => {
                        if little_endian.get() {
                            cx.error_spanned_by(word, format!("attribute meta items `{}` and `{}` are mutually exclusive", BIG_ENDIAN, LITTLE_ENDIAN));
                        } else {
                            big_endian.set_true(word);
                        }
                    }
                    // `#[lain(little_endian)]`
                    Meta(Word(ref word)) if word == LITTLE_ENDIAN => {
                        if big_endian.get() {
                            cx.error_spanned_by(word, format!("attribute meta items `{}` and `{}` are mutually exclusive", BIG_ENDIAN, LITTLE_ENDIAN));
                        } else {
                            little_endian.set_true(word);
                        }
                    }
                    // `#[lain(ignore)]`
                    Meta(Word(ref word)) if word == IGNORE => {
                        ignore.set_true(word);
                    }
                    // `#[lain(ignore_chance = 99.0)]`
                    Meta(NameValue(ref m)) if m.ident == IGNORE_CHANCE => {
                        if let Float(ref f) = m.lit {
                            ignore_chance.set(&m.ident, f.value());
                        } else {
                            cx.error_spanned_by(&m.lit, format!("failed to parse float expression for `{}`", IGNORE_CHANCE));
                        }
                    }
                    Meta(NameValue(ref m)) if m.ident == INITIALIZER => {
                        if let Ok(ref s) = get_lit_str(cx, INITIALIZER, INITIALIZER, &m.lit) {
                            if let Ok(tokens) = TokenStream::from_str(&s.value()) {
                                initializer.set(&m.ident, tokens);
                            } else {
                                cx.error_spanned_by(&m.lit, format!("failed to parse tokens for `{}`", INITIALIZER))
                            }
                        }
                    }
                    Meta(NameValue(ref m)) if m.ident == WEIGHT_TO => {
                        if let Ok(s) = get_lit_str(cx, WEIGHT_TO, WEIGHT_TO, &m.lit) {
                            match s.value().as_ref() {
                                "min" => weight_to.set(&m.ident, WeightTo::Min),
                                "max" => weight_to.set(&m.ident, WeightTo::Max),
                                "none" => weight_to.set(&m.ident, WeightTo::None),
                                _ => cx.error_spanned_by(&m.lit, format!("unknown option `{}` for `{}`", WEIGHT_TO, m.ident))
                            }
                        }
                    }
                    Meta(ref meta_item) => {
                        cx.error_spanned_by(
                            meta_item.name(),
                            format!("unknown lain field attribute `{}`", meta_item.name()),
                        );
                    }
                    Literal(ref lit) => {
                        cx.error_spanned_by(lit, "unexpected literal in lain field attribute");
                    }
                }
            }
        }

        Field {
            name: ident,
            bits: bits.get(),
            bit_shift: None, // this gets fixed up later
            bitfield_type: bitfield_type.get(),
            min: min.get(),
            max: max.get(),
            ignore: ignore.get(),
            ignore_chance: ignore_chance.get(),
            initializer: initializer.get(),
            little_endian: little_endian.get(),
            big_endian: big_endian.get(),
            weight_to: weight_to.get(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bits(&self) -> Option<usize> {
        self.bits.clone()
    }

    pub fn bit_shift(&self) -> Option<usize> {
        self.bit_shift.clone()
    }

    pub fn set_bit_shift(&mut self, shift: usize) {
        self.bit_shift = Some(shift);
    }

    pub fn bitfield_type(&self) -> Option<&syn::Type> {
        self.bitfield_type.as_ref()
    }

    pub fn min(&self) -> Option<&TokenStream> {
        self.min.as_ref()
    }

    pub fn max(&self) -> Option<&TokenStream> {
        self.max.as_ref()
    }

    pub fn ignore(&self) -> bool {
        self.ignore
    }

    pub fn ignore_chance(&self) -> Option<f64> {
        self.ignore_chance.clone()
    }

    pub fn initializer(&self) -> Option<&TokenStream> {
        self.initializer.as_ref()
    }

    pub fn little_endian(&self) -> bool {
        self.little_endian
    }

    pub fn big_endian(&self) -> bool {
        self.big_endian
    }

    pub fn weight_to(&self) -> Option<&WeightTo> {
        self.weight_to.as_ref()
    }
}

/// Represents enum variant information
pub struct Variant {
    weight: Option<u64>,
    ignore: bool,
}

impl Variant {
    /// Extract out the `#[lain()]` attributes from an enum variant
    pub fn from_ast(cx: &Ctxt, variant: &syn::Variant) -> Self {
        let mut weight = Attr::none(cx, WEIGHT);
        let mut ignore = BoolAttr::none(cx, IGNORE);

        for meta_items in variant.attrs.iter().filter_map(get_lain_meta_items) {
            for meta_item in meta_items {
                match meta_item {
                    // `#[lain(weight = 3)]`
                    Meta(NameValue(ref m)) if m.ident == WEIGHT => {
                        if let Int(ref i) = m.lit {
                            weight.set(&m.ident, i.value());
                        } else {
                            cx.error_spanned_by(&m.lit, format!("failed to parse integer expression for {}", WEIGHT));
                        }
                    }
                    // `#[lain(ignore)]`
                    Meta(Word(ref word)) if word == IGNORE => {
                        ignore.set_true(word);
                    }
                    Meta(ref meta_item) => {
                        cx.error_spanned_by(
                            meta_item.name(),
                            format!("unknown lain variant attribute `{}`", meta_item.name()),
                        );
                    }
                    Literal(ref lit) => {
                        cx.error_spanned_by(lit, "unexpected literal in lain variant attribute");
                    }
                }
            }
        }

        Variant {
            weight: weight.get(),
            ignore: ignore.get(),
        }
    }

    pub fn weight(&self) -> Option<u64> {
        self.weight.clone()
    }

    pub fn ignore(&self) -> bool {
        self.ignore
    }
}


pub fn get_lain_meta_items(attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
    if attr.path == LAIN {
        match attr.interpret_meta() {
            Some(List(ref meta)) => Some(meta.nested.iter().cloned().collect()),
            _ => {
                // TODO: produce an error
                None
            }
        }
    } else {
        None
    }
}

pub fn get_lit_str<'a>(cx: &Ctxt, attr_name: Symbol, meta_item_name: Symbol, lit: &'a syn::Lit) -> Result<&'a syn::LitStr, ()> {
    if let syn::Lit::Str(ref lit) = *lit {
        Ok(lit)
    } else {
        cx.error_spanned_by(
            lit,
            format!(
                "expected lain {} attribute to be a string: `{} = \"...\"`",
                attr_name, meta_item_name
            ),
        );
        Err(())
    }
}

/// Parses a `#[lain(min = ..)]` or `#[lain(max = ..)]` attribute
fn parse_min_max(cx: &Ctxt, attr_name: Symbol, lit: &syn::Lit) -> Result<TokenStream, ()> {
    // For a lit str we don't want to emit the tokens as a string, so we
    // reconstruct it as a TokenStream here
    if let syn::Lit::Str(ref s) = lit {
        if let Ok(value) = TokenStream::from_str(&s.value()) {
            Ok(quote_spanned! {lit.span() => #value})
        } else {
            cx.error_spanned_by(lit, format!("invalid tokens for {}", MIN));
            Err(())
        }
    } else if let Int(i) = lit {
        // Reconstruct the int without any suffix. We want to use type
        // inference when we emit the tokens
        let int = LitInt::new(i.value(), IntSuffix::None, lit.span());
        Ok(quote_spanned! {lit.span() => #int})
    } else {
        Ok(quote_spanned! {lit.span() => #lit})
    }
}

fn parse_lit_into_type(
    cx: &Ctxt,
    attr_name: Symbol,
    lit: &syn::Lit,
) -> Result<syn::Type, ()> {
    let string = get_lit_str(cx, attr_name, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        cx.error_spanned_by(lit, format!("failed to parse path: {:?}", string.value()))
    })
}

fn parse_lit_into_expr_path(
    cx: &Ctxt,
    attr_name: Symbol,
    lit: &syn::Lit,
) -> Result<syn::ExprPath, ()> {
    let string = get_lit_str(cx, attr_name, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        cx.error_spanned_by(lit, format!("failed to parse path: {:?}", string.value()))
    })
}

fn parse_lit_str<T>(s: &syn::LitStr) -> parse::Result<T>
where T: Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> parse::Result<TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan_token_stream(stream, s.span()))
}

fn respan_token_stream(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|token| respan_token_tree(token, span))
        .collect()
}

fn respan_token_tree(mut token: TokenTree, span: Span) -> TokenTree {
    if let TokenTree::Group(ref mut g) = token {
        *g = Group::new(g.delimiter(), respan_token_stream(g.stream().clone(), span));
    }
    token.set_span(span);
    token
}