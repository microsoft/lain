use proc_macro2::TokenStream;

use quote::{quote, quote_spanned};

use crate::utils::*;
use syn::spanned::Spanned;
use syn::{Data, Ident};

use std::str::FromStr;

pub(crate) fn get_post_mutation_impl(ident: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                let fields = parse_fields(&fields);

                if fields.is_empty() {
                    return TokenStream::new();
                }

                let mut base_tokens = quote_spanned! { ident.span() => };

                for field in fields {
                    let field_name = &field.field.ident;
                    let field_ty = &field.field.ty;
                    base_tokens.extend(quote_spanned! { field.field.span() =>
                        <#field_ty>::fixup(&mut self.#field_name, mutator);
                    });
                }

                return base_tokens;
            } else {
                panic!("struct contains unnamed fields");
            }
        }
        _ => TokenStream::new(),
    }
}

pub(crate) fn get_post_fuzzer_iteration_impls(ident: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                let fields = parse_fields(&fields);

                if fields.is_empty() {
                    return TokenStream::new();
                }

                let mut base_tokens = quote_spanned!(ident.span() => );

                for field in fields {
                    let field_name = field.field.ident;
                    let field_type = &field.field.ty;
                    base_tokens.extend(quote_spanned! { field.field.span() =>
                        <#field_type>::on_success(&self.#field_name);
                    });
                }

                return base_tokens;
            } else {
                panic!("struct contains unnamed fields");
            }
        }
        _ => TokenStream::new(),
    }
}

pub(crate) fn gen_mutate_impl(ident: &Ident, data: &Data) -> TokenStream {
    let mutate_body: TokenStream;

    match *data {
        Data::Enum(ref data) => {
            let enum_ident = ident.to_string();

            let mut enum_has_non_unit_variants = false;
            let mut mutate_match_arms: Vec<TokenStream> = Vec::new();
            for variant in &data.variants {
                let variant_ident = TokenStream::from_str(&format!(
                    "{}::{}",
                    enum_ident,
                    variant.ident.to_string()
                ))
                .unwrap();

                match variant.fields {
                    syn::Fields::Unnamed(ref fields) => {
                        let mut parameters = TokenStream::new();
                        let mut mutate_call = TokenStream::new();

                        for (i, ref unnamed) in fields.unnamed.iter().enumerate() {
                            let field_ty = &unnamed.ty;
                            let ident = TokenStream::from_str(&format!("field_{}", i)).unwrap();

                            mutate_call.extend(quote_spanned! { unnamed.span() =>
                                let constraints = max_size.and_then(|max|{
                                    let mut c = ::lain::types::Constraints::new();
                                    c.base_object_size_accounted_for = true;
                                    c.max_size(max);

                                    Some(c)
                                });

                                <#field_ty>::m(#ident, mutator, constraints.as_ref());
                                if <#field_ty>::is_variable_size() {
                                    max_size = max_size.map(|max| {
                                        // in case a user didn't appropriately supply a max size constraint (i.e. a max
                                        // size that's smaller than the object's min size), we don't want to panic
                                        let serialized_size = #ident.serialized_size();

                                        if serialized_size > max {
                                            warn!("Max size provided to object is likely smaller than min object size");

                                            0
                                        } else {
                                            max - serialized_size
                                        }
                                    });
                                }
                            });

                            parameters.extend(quote_spanned! {unnamed.span() => ref mut #ident,});
                        }

                        mutate_match_arms.push(quote! {
                            #variant_ident(#parameters) => {
                                #mutate_call
                            },
                        });
                        enum_has_non_unit_variants = true;
                    }
                    syn::Fields::Unit => {
                        break;
                    }
                    _ => panic!("unsupported enum variant type"),
                }
            }

            mutate_body = if !enum_has_non_unit_variants {
                // TODO: This will keep any #[fuzzer(ignore)] or #[weight(N)] attributes...
                // which we probably don't want.
                quote_spanned! { ident.span() =>
                    *self = <#ident>::new_fuzzed(mutator, parent_constraints);
                }
            } else {
                quote_spanned! { ident.span() =>
                    // Make a copy of the constraints that will remain immutable for
                    // this function. Here we ensure that the base size of this object has
                    // been accounted for by the caller, which may be an object containing this.
                    let parent_constraints = parent_constraints.and_then(|c| {
                        let mut c = c.clone();
                        c.account_for_base_object_size::<Self>();

                        Some(c)
                    });

                    let mut max_size = parent_constraints.as_ref().and_then(|c| c.max_size);

                    match *self {
                        #(#mutate_match_arms)*
                    }
                }
            };
        }
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                let fields = parse_fields(&fields);
                mutate_body = gen_struct_mutate_impl(&fields);
            } else {
                panic!("struct contains unnamed fields");
            }
        }
        Data::Union(ref _data) => {
            panic!("unions are unsupported. Please use an enum with typed variants instead");
        }
    }

    quote_spanned! { ident.span() =>
        #[allow(unused)]
        fn mutate<R: ::lain::rand::Rng>(&mut self, mutator: &mut ::lain::mutator::Mutator<R>, parent_constraints: Option<&::lain::types::Constraints<u8>>) {
            #mutate_body

            if mutator.should_fixup() {
                self.fixup(mutator);
            }
        }
    }
}

fn gen_struct_mutate_impl(fields: &[FuzzerObjectStructField]) -> TokenStream {
    let mutation_parts: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let mut field_mutation_tokens = TokenStream::new();
            let span = f.field.span();
            let ty = &f.field.ty;
            let ident = &f.field.ident;
            let ident_str = ident.as_ref().unwrap().to_string();
            let weighted = &f.weighted;

            let default_constraints = if f.min.is_some() || f.max.is_some() {
                let min = f
                    .min
                    .as_ref()
                    .map(|v| quote! {Some(#v)})
                    .unwrap_or_else(|| quote! {None});
                let max = f
                    .max
                    .as_ref()
                    .map(|v| quote! {Some(#v)})
                    .unwrap_or_else(|| quote! {None});

                quote_spanned! { span =>
                    let mut constraints = Constraints::new();
                    constraints.min = #min;
                    constraints.max = #max;
                    constraints.max_size = max_size.clone();
                    constraints.weighted = #weighted;
                    constraints.base_object_size_accounted_for = true;

                    let constraints = Some(constraints);
                }
            } else {
                quote_spanned! { span =>
                    let constraints = max_size.and_then(|max|{
                        let mut c = ::lain::types::Constraints::new();
                        c.base_object_size_accounted_for = true;
                        c.max_size(max);

                        Some(c)
                    });
                }
            };

            field_mutation_tokens.extend(quote! {
                #default_constraints
                <#ty>::mutate(&mut self.#ident, mutator, constraints.as_ref());
                if <#ty>::is_variable_size() {
                    max_size = max_size.map(|max| {
                        // in case a user didn't appropriately supply a max size constraint (i.e. a max
                        // size that's smaller than the object's min size), we don't want to panic
                        let serialized_size = self.#ident.serialized_size();

                        if serialized_size > max {
                            warn!("Max size provided to object is likely smaller than min object size");

                            0
                        } else {
                            max - serialized_size
                        }
                    });
                }

                if mutator.should_early_bail_mutation() {
                    if mutator.should_fixup() {
                        <#ty>::fixup(&mut self.#ident, mutator);
                    }

                    return;
                }
            });

            field_mutation_tokens
        })
        .collect();

    quote! {
        // Make a copy of the constraints that will remain immutable for
        // this function. Here we ensure that the base size of this object has
        // been accounted for by the caller, which may be an object containing this.
        let parent_constraints = parent_constraints.and_then(|c| {
            let mut c = c.clone();
            c.account_for_base_object_size::<Self>();

            Some(c)
        });

        let mut max_size = parent_constraints.as_ref().and_then(|c| c.max_size);

        #(#mutation_parts)*
    }
}
