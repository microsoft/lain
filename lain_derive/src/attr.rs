
use proc_macro2::TokenStream;
use std::str::FromStr;
use syn::Meta::{List, NameValue};
use syn::NestedMeta::Meta;

#[derive(Default)]
pub struct BitfieldMetadata {
    pub ty: Option<TokenStream>,
    pub ty_bits: usize,
    pub min: u64,
    pub max: u64,
    pub bit_count: usize,
}

pub(crate) fn get_bitfield_limits(
    meta: impl Iterator<Item = Vec<syn::NestedMeta>>,
) -> Option<BitfieldMetadata> {
    let mut bm = BitfieldMetadata::default();

    for meta_items in meta {
        for meta_item in meta_items {
            match meta_item {
                Meta(NameValue(ref m)) if m.ident == "backing_type" => {
                    let lit_str = get_lit_str(&m.lit).unwrap().value();
                    match lit_str.as_ref() {
                        "u8" => {
                            bm.ty_bits = 8;
                        }
                        "u16" => {
                            bm.ty_bits = 16;
                        }
                        "u32" => {
                            bm.ty_bits = 32;
                        }
                        "u64" => {
                            bm.ty_bits = 64;
                        }
                        _ => {
                            panic!("unsupported backing type for bitfield -- must be u8, u16, u32, or u64");
                        }
                    }

                    bm.ty = Some(TokenStream::from_str(&lit_str).unwrap());
                }
                Meta(NameValue(ref m)) if m.ident == "bits" => {
                    let bit_count = get_lit_number(&m.lit).unwrap().value();
                    if bit_count > 64 {
                        panic!("bit count is larger than 64");
                    }

                    bm.bit_count = bit_count as usize;
                    bm.max = 2_u64.pow(bit_count as u32);
                }
                _ => {
                    panic!(
                        "unexpected meta type in get_bitfield_limits -- should be literals only"
                    );
                }
            }
        }
    }

    if (bm.ty.is_some() && bm.bit_count == 0) || (bm.bit_count > 0 && bm.ty.is_none()) {
        panic!("#[bitfield] requires type and bits to be supplied. e.g. #[bitfield(backing_type=u32, bits=10)]")
    }

    if bm.ty.is_some() {
        return Some(bm);
    }

    None
}

pub(crate) fn get_bitfield_metadata(attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
    get_attribute_metadata("bitfield", &attr)
}

pub(crate) fn get_attribute_metadata(
    ident: &'static str,
    attr: &syn::Attribute,
) -> Option<Vec<syn::NestedMeta>> {
    if attr.path.segments.len() == 1 && attr.path.segments[0].ident == ident {
        match attr.interpret_meta() {
            Some(List(ref meta)) => {
                return Some(meta.nested.iter().cloned().collect());
            }
            _ => return None,
        }
    }

    None
}

pub(crate) fn get_lit_number(lit: &syn::Lit) -> Result<&syn::LitInt, ()> {
    if let syn::Lit::Int(ref lit) = *lit {
        return Ok(lit);
    }
    // TODO: proper errors
    Err(())
}

pub(crate) fn get_lit_str(lit: &syn::Lit) -> Result<&syn::LitStr, ()> {
    if let syn::Lit::Str(ref lit) = *lit {
        return Ok(lit);
    }
    // TODO: proper errors
    Err(())
}

pub(crate) fn get_lit_bool(lit: &syn::Lit) -> Result<&syn::LitBool, ()> {
    if let syn::Lit::Bool(ref lit) = *lit {
        return Ok(lit);
    }
    // TODO: proper errors
    Err(())
}

pub(crate) fn get_fuzzer_metadata(attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
    get_attribute_metadata("fuzzer", &attr)
}
