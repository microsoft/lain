use proc_macro2::{Ident, Span, TokenStream};

use syn;
use crate::internals::attr::unraw;
use quote::quote;

/// Wraps the code in a dummy const object. See https://github.com/serde-rs/serde/issues/159#issuecomment-214002626
pub fn wrap_in_const(
    trait_: &str,
    ty: &Ident,
    code: TokenStream,
) -> TokenStream {

    let dummy_const = Ident::new(
        &format!("_IMPL_{}_FOR_{}", trait_, unraw(ty)),
        Span::call_site(),
    );

    let use_lain = quote! {
        #[allow(unknown_lints)]
        #[cfg_attr(feature = "cargo-clippy", allow(useless_attribute))]
        #[allow(rust_2018_idioms)]
        use ::lain as _lain;
    };

    quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            #use_lain
            #code
        };
    }
}