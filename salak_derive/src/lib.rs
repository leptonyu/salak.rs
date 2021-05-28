//! `salak_derive` provides a derive macro [`FromEnvironment`] for [salak](https://crates.io/crates/salak).
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]
use proc_macro::TokenStream;
use quote::quote;
use syn::*;

fn parse_path(path: Path) -> String {
    path.segments.first().unwrap().ident.to_string()
}

fn parse_lit(lit: Lit) -> String {
    match lit {
        Lit::Str(s) => s.value(),
        Lit::ByteStr(s) => match String::from_utf8(s.value()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        },
        Lit::Int(i) => i.base10_digits().to_owned(),
        Lit::Float(f) => f.base10_digits().to_owned(),
        Lit::Bool(b) => b.value.to_string(),
        Lit::Char(c) => c.value().to_string(),
        Lit::Byte(b) => (b.value() as char).to_string(),
        Lit::Verbatim(_) => panic!("Salak not support Verbatim"),
    }
}

fn parse_attribute_prefix(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if let Ok(Meta::List(list)) = attr.parse_meta() {
            if !is_salak(&list) {
                continue;
            }
            for m in list.nested {
                if let NestedMeta::Meta(Meta::NameValue(nv)) = m {
                    if parse_path(nv.path) == "prefix" {
                        match nv.lit {
                            Lit::Str(s) => return Some(s.value()),
                            _ => panic!("Only support string"),
                        }
                    } else {
                        panic!("Only support prefix");
                    }
                } else {
                    panic!("Only support prefix=\"xxx\"");
                }
            }
        }
    }
    None
}

fn disable_attribute_prefix_enum(attrs: &[Attribute]) {
    for attr in attrs {
        if let Ok(Meta::List(list)) = attr.parse_meta() {
            if !is_salak(&list) {
                continue;
            }
            panic!("Salak attribute is not supporting enum");
        }
    }
}

fn is_salak(list: &MetaList) -> bool {
    if let Some(v) = list.path.segments.iter().next() {
        return v.ident == "salak";
    }
    false
}

fn parse_field_attribute(
    attrs: Vec<Attribute>,
    name: &mut Ident,
) -> (quote::__private::TokenStream, quote::__private::TokenStream) {
    let mut def = None;
    let mut rename = None;
    let mut desc = None;
    for attr in attrs {
        if let Ok(Meta::List(list)) = attr.parse_meta() {
            if !is_salak(&list) {
                continue;
            }
            for m in list.nested {
                if let NestedMeta::Meta(Meta::NameValue(nv)) = m {
                    match &parse_path(nv.path)[..] {
                        "default" => def = Some(parse_lit(nv.lit)),
                        "name" => rename = Some(parse_lit(nv.lit)),
                        "desc" => desc = Some(parse_lit(nv.lit)),
                        _ => panic!("Only support default/name/desc"),
                    }
                } else {
                    panic!("Only support NestedMeta::Meta(Meta::NameValue)");
                }
            }
        }
    }
    if let Some(rename) = rename {
        *name = quote::format_ident!("{}", rename);
    }

    let (a, b) = match def {
        Some(def) => (
            quote! {
                Some(Property::S(#def))
            },
            quote! {
                Some(false), Some(#def)
            },
        ),
        _ => (
            quote! {
                None
            },
            quote! {
                None, None
            },
        ),
    };

    (
        a,
        if let Some(desc) = desc {
            quote! {
                #b, Some(#desc.to_string())
            }
        } else {
            quote! {
                #b, None
            }
        },
    )
}

fn derive_field(field: Field) -> (quote::__private::TokenStream, quote::__private::TokenStream) {
    let name = field.ident.expect("Not possible");
    let ty = field.ty;
    let mut rename = name.clone();
    let (def, def_desc) = parse_field_attribute(field.attrs, &mut rename);
    (
        quote! {
            #name: env.require_def::<#ty, &str>(key, stringify!(#rename), #def)?
        },
        quote! {
            env.key_desc::<#ty, &str>(key, stringify!(#rename), #def_desc, keys);
        },
    )
}

fn derive_fields(
    fields: Fields,
) -> (
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
) {
    if let Fields::Named(fields) = fields {
        let mut v = vec![];
        let mut d = vec![];
        for field in fields.named {
            let (a, b) = derive_field(field);
            v.push(a);
            d.push(b);
        }
        return (v, d);
    }
    panic!("Only support named body");
}

fn derive_struct(name: &Ident, data: DataStruct) -> quote::__private::TokenStream {
    let (field, field_desc) = derive_fields(data.fields);
    quote! {
        impl FromEnvironment for #name {
            fn from_env<'a>(
                key: &mut Key<'a>,
                val: Option<Property<'_>>,
                env: &'a impl SalakContext<'a>,
            ) -> Result<Self, PropertyError> {
                Ok(Self {
                   #(#field),*
                })
            }

            fn key_desc<'a>(
                key: &mut Key<'a>,
                _: &mut KeyDesc,
                keys: &mut Vec<KeyDesc>,
                env: &'a impl SalakContext<'a>,
            ) {
                #(#field_desc)*
            }
        }
    }
}

fn derive_enum(type_name: &Ident, data: &DataEnum) -> quote::__private::TokenStream {
    let mut vs = vec![];
    for variant in &data.variants {
        disable_attribute_prefix_enum(&variant.attrs);
        let lname = quote::format_ident!("{}", format!("{}", variant.ident).to_lowercase());
        let name = &variant.ident;
        let body = match variant.fields {
            Fields::Unit => {
                quote! {
                    stringify!(#lname) => Ok(#type_name::#name),
                }
            }
            _ => panic!("Enum only support no field pattern."),
        };
        vs.push(body);
    }
    quote! {
        impl EnumProperty for #type_name {
            #[inline]
            fn str_to_enum(val: &str) -> Result<#type_name, PropertyError>{
            match &val.to_lowercase()[..] {
                #(#vs)*
                _ => Err(PropertyError::parse_fail("invalid enum value")),
            }
            }
        }
    }
}

/// Derive [FromEnvironment](https://docs.rs/salak/latest/salak/trait.Environment.html).
#[proc_macro_derive(FromEnvironment, attributes(salak))]
pub fn from_env_derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (head, body) = match input.data {
        Data::Struct(d) => (
            if let Some(prefix) = parse_attribute_prefix(&input.attrs) {
                quote! {
                        impl PrefixedFromEnvironment for #name {
                        fn prefix() -> &'static str {
                            #prefix
                        }
                    }
                }
            } else {
                quote! {}
            },
            derive_struct(&name, d),
        ),
        Data::Enum(d) => {
            disable_attribute_prefix_enum(&input.attrs);
            (quote! {}, derive_enum(&name, &d))
        }
        _ => panic!("union is not supported"),
    };

    TokenStream::from(quote! {
        impl AutoDeriveFromEnvironment for #name {}
        #head
        #body
    })
}
