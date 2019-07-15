extern crate proc_macro;

use proc_macro2::TokenStream;

use quote::quote;

use std::str::FromStr;
use syn::Meta::Word;
use syn::NestedMeta::Meta;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Type};

use crate::attr::*;
use crate::utils::*;

#[derive(Default)]
struct BinarySerializeTokens {
    pub serialize: TokenStream,
    pub serialized_size: Option<TokenStream>,
    pub min_nonzero_elements_size: Option<TokenStream>,
}

impl BinarySerializeTokens {
    fn new(
        serialize: TokenStream,
        serialized_size: Option<TokenStream>,
        min_nonzero_elements_size: Option<TokenStream>,
    ) -> BinarySerializeTokens {
        BinarySerializeTokens {
            serialize,
            serialized_size,
            min_nonzero_elements_size,
        }
    }
}

pub(crate) fn binary_serialize_helper(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let mut use_inner_member_serialized_size = true;
    let mut static_serialized_size: Option<usize> = None;

    if !input.attrs.is_empty() {
        for attr in &input.attrs {
            if attr.path.segments[0].ident == "serialized_size" {
                let meta = attr.parse_meta().expect("couldn't parse meta");
                if let syn::Meta::List(l) = meta {
                    if l.nested.len() > 1 {
                        panic!("only expected 1 item in serialized_size");
                    }

                    let nested = &l.nested[0];
                    if let syn::NestedMeta::Literal(lit) = nested {
                        if let syn::Lit::Int(int) = lit {
                            static_serialized_size = Some(int.value() as usize);
                            use_inner_member_serialized_size = false;
                        }
                    }
                }
            }
        }
    }

    let name = input.ident;
    let name_as_string = name.to_string();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let tokens = serialize_fields(&name, &input.data, use_inner_member_serialized_size);

    let serialize = tokens.serialize;

    let serialized_size = if let Some(size) = static_serialized_size {
        quote! {#size}
    } else if let Some(ref serialized_size) = tokens.serialized_size {
        serialized_size.clone()
    } else {
        quote! {std::mem::size_of_val(&self.to_primitive());}
    };

    let min_nonzero_elements_size = tokens.min_nonzero_elements_size.unwrap();

    let serialized_size = quote! {
        impl #impl_generics ::lain::traits::SerializedSize for #name #ty_generics #where_clause {
            #[inline(always)]
            fn serialized_size(&self) -> usize {
                use ::lain::traits::SerializedSize;
                ::lain::log::debug!("getting serialized size of {}", #name_as_string);
                let size = #serialized_size;
                ::lain::log::debug!("size of {} is 0x{:02X}", #name_as_string, size);

                return size;
            }

            #[inline(always)]
            fn min_nonzero_elements_size() -> usize {
                #min_nonzero_elements_size
            }
        }
    };

    // println!("{}", serialized_size);

    let expanded = quote! {
        impl #impl_generics ::lain::traits::BinarySerialize for #name #ty_generics #where_clause {
            fn binary_serialize<W: std::io::Write, E: ::lain::byteorder::ByteOrder>(&self, buffer: &mut W) {
                use ::lain::traits::SerializedSize;
                use ::lain::byteorder::{LittleEndian, BigEndian, WriteBytesExt};

                #serialize
            }
        }

        #serialized_size
    };

    // Uncomment to dump the AST
    // println!("{}", expanded);

    proc_macro::TokenStream::from(expanded)
}

fn serialize_fields(
    name: &Ident,
    data: &Data,
    use_inner_member_serialized_size: bool,
) -> BinarySerializeTokens {
    match *data {
        Data::Enum(ref data) => {
            let mut variant_branches = Vec::<TokenStream>::new();
            let mut serialized_size_variant_branches = Vec::<TokenStream>::new();
            let mut min_sizes = Vec::<TokenStream>::new();

            for variant in data.variants.iter() {
                let ident = &variant.ident;
                let full_ident_string = format!("{}::{}", &name.to_string(), &ident.to_string());

                let full_ident = TokenStream::from_str(&full_ident_string).unwrap();
                let mut parameters = TokenStream::new();

                match variant.fields {
                    syn::Fields::Unnamed(ref fields) => {
                        let mut serialized_fields = TokenStream::new();
                        let mut total_size = TokenStream::new();
                        let mut variant_sizes = Vec::<TokenStream>::new();

                        // iterate over every item in this tuple
                        for (i, ref unnamed) in fields.unnamed.iter().enumerate() {
                            let field_ty = &unnamed.ty;
                            let ident = TokenStream::from_str(&format!("field_{}", i)).unwrap();

                            total_size.extend(quote! {total_size += #ident.serialized_size();});
                            variant_sizes.push(quote! {std::mem::size_of::<#field_ty>()});

                            serialized_fields.extend(quote! {
                                #ident.binary_serialize::<_, E>(buffer);
                            });

                            parameters.extend(quote! {ref #ident,});
                        }
                        min_sizes.push(quote! {0#(+#variant_sizes)*});

                        let serialized_size = if use_inner_member_serialized_size {
                            quote! {
                                let serialized_size = total_size;
                            }
                        } else {
                            quote! {
                                let serialized_size = self.serialized_size();
                            }
                        };

                        let trailer = quote! {
                            let padding = serialized_size - total_size;
                            if padding != 0 {
                                let padding_data: Vec<u8> = (0..padding).map(|_| 0).collect();
                                buffer.write(&padding_data).ok();
                            }
                        };

                        variant_branches.push(quote!{
                            #full_ident(#parameters) => {
                                let mut total_size = 0;
                                #total_size
                                #serialized_size
                                // TODO: we technically handle multiple fields, but this is hardcoded
                                if total_size > serialized_size {
                                    panic!("size of serialized data for {} will be greater than enum's marked size ({} > {})", #full_ident_string, total_size, serialized_size);
                                }

                                #serialized_fields

                                #trailer
                            },
                        });

                        serialized_size_variant_branches.push(quote! {
                            #full_ident(#parameters) => {
                                let mut total_size = 0;
                                #total_size
                                #serialized_size

                                serialized_size
                            },
                        });
                        //println!("{}", variant_branches[variant_branches.len() - 1]);
                    }
                    syn::Fields::Unit => {
                        let serialize = quote! {
                            self.to_primitive().binary_serialize::<_, E>(buffer);
                        };

                        let size = quote! {
                            std::mem::size_of::<#name>();
                        };

                        let min_size = quote! {
                            std::mem::size_of::<#name>()
                        };

                        return BinarySerializeTokens::new(serialize, Some(size), Some(min_size));
                    }
                    _ => panic!("unsupported enum type (probably contains named members)"),
                }
            }

            let serialize_body = quote! {
                match *self {
                    #(#variant_branches)*
                }
            };

            let serialized_size_body = quote! {
                match *self {
                    #(#serialized_size_variant_branches)*
                }
            };

            let sizes = quote! {*[#(#min_sizes,)*].iter().min_by(|a, b| a.cmp(b)).unwrap()};

            BinarySerializeTokens::new(serialize_body, Some(serialized_size_body), Some(sizes))
        }
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let mut bitfield_shift = 0;
                    let mut bitfield_type: Option<TokenStream> = None;

                    let fields = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;

                        // parse out the byteorder
                        let meta = f.attrs.iter().filter_map(get_byteorder_metadata);
                        let field_byteorder = get_byteorder(meta);

                        let meta = f.attrs.iter().filter_map(get_bitfield_metadata);


                        // this is a bitfield. we need to use our "bitfield" local variable
                        // to temporarily hold all these bits
                        let bitfield_meta = get_bitfield_limits(meta);
                        let is_bitfield = bitfield_meta.is_some();

                        if is_bitfield {
                            let primitive_type = match ty {
                                Type::Path(ref p) if !p.path.segments.is_empty() => {
                                    let base_type = p.path.segments[0].ident.to_string();
                                    is_primitive(&base_type)
                                }
                                _ => {
                                    panic!("bitfields are only supported for paths -- arrays should not be used");
                                }
                            };

                            let bitfield_meta = bitfield_meta.unwrap();
                            let old_shift = bitfield_shift;
                            let num_bits = bitfield_meta.bit_count;

                            bitfield_shift += num_bits;
                            bitfield_type = bitfield_meta.ty;

                            let bit_mask = 2_u64.pow(num_bits as u32) - 1;

                            // TODO: Min/max validation
                            let mut text = match primitive_type {
                                PrimitiveType::Number => {
                                    quote! {
                                        bitfield |= ((self.#name as #bitfield_type & #bit_mask as #bitfield_type) << #old_shift) as u64;
                                    }
                                }
                                _ => {
                                    quote! {
                                        bitfield |= (((self.#name.to_primitive() as u64) & (#bit_mask as u64)) << #old_shift) as u64;
                                    }
                                }
                            };

                            if bitfield_meta.ty_bits == bitfield_shift {
                                text.extend(quote!{
                                    (bitfield as #bitfield_type).binary_serialize::<_, E>(buffer);
                                    bitfield = 0;
                                });

                                bitfield_shift = 0;
                            }

                            let size = if old_shift == 0 {
                                Some(quote! {
                                    std::mem::size_of::<#bitfield_type>()
                                })
                            } else {
                                None
                            };

                            return BinarySerializeTokens::new(text, size.clone(), size);
                        }

                        fn handle_type(name: &syn::Ident, ty: &syn::Type, field_byteorder: Option<&TokenStream>) -> BinarySerializeTokens {
                            let handle_ident =  |ty: &syn::Path| {
                                let root = &ty.segments[0].ident;
                                let primitive_type = is_primitive(&root.to_string());

                                let size = match primitive_type {
                                    PrimitiveType::Number | PrimitiveType::Bool => {
                                        quote! {
                                            std::mem::size_of::<#ty>()
                                        }
                                    },
                                    _ => {
                                        quote! {
                                            self.#name.serialized_size()
                                        }
                                    }
                                };

                                let min_size = match primitive_type {
                                    PrimitiveType::Number | PrimitiveType::Bool => {
                                        quote! {
                                            std::mem::size_of::<#ty>()
                                        }
                                    },
                                    _ => {
                                        quote! {
                                            <#ty>::min_nonzero_elements_size()
                                        }
                                    }
                                };


                                // If the field has a custom byteorder to override however the parent
                                // is serialized, we handle that here
                                if field_byteorder.is_some() {
                                    let binary_serialize_text = quote!{
                                        self.#name.binary_serialize::<_, #field_byteorder>(buffer);
                                    };

                                    return BinarySerializeTokens::new(binary_serialize_text, Some(size), None);
                                }

                                let binary_serialize_text = quote!{
                                    self.#name.binary_serialize::<_, E>(buffer);
                                };

                                BinarySerializeTokens::new(binary_serialize_text, Some(size), Some(min_size))
                            };

                            match ty {
                                Type::Path(ref p) if !p.path.segments.is_empty() => {
                                    handle_ident(&p.path)
                                }
                                Type::Array(a) => {
                                    let array_len = &a.len;
                                    let mut tokens = handle_type(&name, &a.elem, field_byteorder);
                                    let per_item_serialized_size = tokens.serialized_size;
                                    let per_item_min = tokens.min_nonzero_elements_size;

                                    // use fold to avoid breaking the line
                                    tokens.serialized_size = Some(quote! {
                                        self.#name.iter().fold(0, |sum, i| sum + #per_item_serialized_size)
                                    });

                                    tokens.min_nonzero_elements_size = Some(quote!{
                                        #per_item_min * #array_len
                                    });

                                    tokens
                                },
                                Type::Reference(ref reference) => {
                                    handle_type(&name, &reference.elem, field_byteorder)
                                }
                                _ => {
                                    panic!("Unsupported type");
                                }
                            }
                        }

                        handle_type(&name.as_ref().unwrap(), &ty, field_byteorder.as_ref())
                    });

                    let mut serialize_text = quote! {
                       // may not be used in all scenarios
                       let mut bitfield: u64 = 0;
                    };

                    let mut object_size = quote! {0};
                    let mut min_object_size = quote! {0};

                    for item in fields {
                        serialize_text.extend(item.serialize);

                        let item_size = item.serialized_size;
                        if item_size.is_some() {
                            object_size.extend(quote! {
                                + #item_size
                            });
                        }

                        if let Some(ref min_size) = item.min_nonzero_elements_size {
                            min_object_size.extend(quote! {
                                + #min_size
                            });
                        }
                    }

                    // the above case would not push the byte if there is
                    // a bitfield in the final position with padding
                    if bitfield_shift != 0 {
                        serialize_text.extend(quote! {
                            (bitfield as #bitfield_type).binary_serialize::<_, E>(buffer);
                            bitfield = 0;
                        });
                    }

                    BinarySerializeTokens::new(
                        serialize_text,
                        Some(object_size),
                        Some(min_object_size),
                    )
                }
                _ => {
                    panic!("BinarySerializer only supports named fields");
                }
            }
        }
        _ => {
            panic!("BinarySerializer is only supported for structs");
        }
    }
}

/// Returns the user-specified byteorder of a child field based off of the #[byteorder()] attribute.
/// This will return an Option<TokenStream> consisting of the full path to the byteorder::BigEndian or
/// byteorder::LittleEndian enum.
fn get_byteorder(meta: impl Iterator<Item = Vec<syn::NestedMeta>>) -> Option<TokenStream> {
    for meta_items in meta {
        for meta_item in meta_items {
            match meta_item {
                Meta(ref m) => match m {
                    Word(ref w) => {
                        match w.to_string().as_ref() {
                            "big" => return Some(quote! {::lain::byteorder::BigEndian}),
                            "little" => return Some(quote! {::lain::byteorder::LittleEndian}),
                            _ => panic!(
                                "{} is not a supported byteorder. must be big or little",
                                w.to_string()
                            ),
                        };
                    }
                    _ => panic!("non-string literal for byteorder attribute"),
                },
                _ => panic!(
                    "#[byteorder] attribute expects a string literal (e.g. #[byteorder(big)]"
                ),
            }
        }
    }
    None
}

fn get_byteorder_metadata(attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
    get_attribute_metadata("byteorder", &attr)
}
