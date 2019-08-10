use super::attr;
use super::Derive;
use super::Ctxt;
use syn::punctuated::Punctuated;
use syn::Token;

/// A source data structure annotated with `#[derive(NewFuzzed)]` and/or `#[derive(Mutatable)]`
pub struct Container<'a> {
    /// The struct or enum name (without generics).
    pub ident: syn::Ident,
    /// Attributes on the container, parsed for lain.
    pub attrs: attr::Container,
    /// The contents of the struct or enum.
    pub data: Data<'a>,
    /// Any generics on the struct or enum.
    pub generics: &'a syn::Generics,
    /// Original input.
    pub original: &'a syn::DeriveInput,
}

/// The fields of a struct or enum
/// 
/// Analagous to [`syn::Data`].
pub enum Data<'a> {
    Enum(Vec<Variant<'a>>),
    Struct(Style, Vec<Field<'a>>),
}

/// A variant of an enum.
pub struct Variant<'a> {
    pub ident: syn::Ident,
    pub attrs: attr::Variant,
    pub style: Style,
    pub fields: Vec<Field<'a>>,
    pub original: &'a syn::Variant,
}

/// A field of a struct.
pub struct Field<'a> {
    pub member: syn::Member,
    pub attrs: attr::Field,
    pub ty: &'a syn::Type,
    pub original: &'a syn::Field,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Style {
    /// Named fields.
    Struct,
    /// Many unnamed fields.
    Tuple,
    /// No fields.
    Unit,
}

impl<'a> Container<'a> {
    /// Convert the raw syn AST into a parsed container object, collecting errors in `cx`
    pub fn from_ast(
        cx: &Ctxt,
        item: &'a syn::DeriveInput,
        derive: Derive,
    ) -> Option<Container<'a>> {
        let mut attrs = attr::Container::from_ast(cx, item);

        let mut data = match item.data {
            syn::Data::Enum(ref data) => {
                Data::Enum(enum_from_ast(cx, &data.variants))
            }
            syn::Data::Struct(ref data) => {
                let (style, fields) = struct_from_ast(cx, &data.fields);
                Data::Struct(style, fields)
            }
            syn::Data::Union(_) => {
                cx.error_spanned_by(item, "lain does not support derive for unions");
                return None;
            }
        };

        let mut item = Container {
            ident: item.ident.clone(),
            attrs,
            data,
            generics: &item.generics,
            original: item,
        };

        Some(item)
    }
}

impl<'a> Data<'a> {
    pub fn all_fields(&'a self) -> Box<dyn Iterator<Item = &'a Field<'a>> + 'a> {
        match *self {
            Data::Enum(ref variants) => {
                Box::new(variants.iter().flat_map(|variant| variant.fields.iter()))
            }
            Data::Struct(_, ref fields) => Box::new(fields.iter()),
        }
    }
}

fn enum_from_ast<'a>(
    cx: &Ctxt,
    variants: &'a Punctuated<syn::Variant, Token![,]>,
) -> Vec<Variant<'a>> {
    variants.iter().map(|variant| {
        let attrs = attr::Variant::from_ast(cx, variant);
        let (style, fields) = struct_from_ast(cx, &variant.fields);

        Variant {
            ident: variant.ident.clone(),
            attrs,
            style,
            fields,
            original: variant,
        }
    }).collect()
}

fn struct_from_ast<'a>(
    cx: &Ctxt,
    fields: &'a syn::Fields,
) -> (Style, Vec<Field<'a>>) {
    match *fields {
        syn::Fields::Named(ref fields) => (
            Style::Struct,
            fields_from_ast(cx, &fields.named)
        ),
        syn::Fields::Unnamed(ref fields) => (
            Style::Tuple,
            fields_from_ast(cx, &fields.unnamed)
        ),
        syn::Fields::Unit => (Style::Unit, Vec::new()),
    }
}

fn fields_from_ast<'a>(
    cx: &Ctxt,
    fields: &'a Punctuated<syn::Field, Token![,]>,
) -> Vec<Field<'a>> {
    let mut bitfield_bits = 0;

    fields
    .iter()
    .enumerate()
    .map(|(i, field)| {
        let mut field = Field {
            member: match field.ident {
                Some(ref ident) => syn::Member::Named(ident.clone()),
                None => syn::Member::Unnamed(i.into()),
            },
            attrs: attr::Field::from_ast(cx, i, field),
            ty: &field.ty,
            original: field,
        };

        if let Some(bits) = field.attrs.bits() {
            field.attrs.set_bit_shift(bitfield_bits);
            bitfield_bits += bits;

            let mut bits_in_type = 0;
            
            let bitfield_type = field.attrs.bitfield_type().unwrap_or(&field.ty);
            if is_primitive_type(bitfield_type, "u8") {
                bits_in_type = 8
            } else if is_primitive_type(bitfield_type, "u16") {
                bits_in_type = 16
            } else if is_primitive_type(bitfield_type, "u32") {
                bits_in_type = 32
            } else if is_primitive_type(bitfield_type, "u64") {
                bits_in_type = 64
            } else {
                cx.error_spanned_by(&field.ty, "Unsupported bitfield datatype. Did you forget to specify `#[lain(backing_type = \"...\")]`?");
                return field;
            }

            if bitfield_bits == bits_in_type {
                bitfield_bits = 0;
            } else if bitfield_bits > bits_in_type {
                cx.error_spanned_by(&field.ty, "Number of bits specified overflows bitfield type");
            }
        }

        field
    })
    .collect()
}

pub fn is_primitive_type(ty: &syn::Type, primitive: &str) -> bool {
    match *ty {
        syn::Type::Path(ref ty) => ty.qself.is_none() && is_primitive_path(&ty.path, primitive),
        _ => false,
    }
}

fn is_primitive_path(path: &syn::Path, primitive: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments[0].ident == primitive
        && path.segments[0].arguments.is_empty()
}