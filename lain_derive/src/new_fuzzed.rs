
use crate::utils::*;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Lit, NestedMeta};

use crate::internals::Ctxt;

use crate::attr::{get_attribute_metadata, get_fuzzer_metadata, get_lit_bool};

pub fn expand_new_fuzzed(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Ctxt::new();
    
}