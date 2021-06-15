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
            #name: env.require_def::<#ty>(stringify!(#rename), #def)?
        },
        quote! {
            env.add_key_desc::<#ty>(stringify!(#rename), #def_desc);
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
            fn from_env(
                val: Option<Property<'_>>,
                env: &mut SalakContext<'_>,
            ) -> Result<Self, PropertyError> {
                Ok(Self {
                   #(#field),*
                })
            }
        }

        impl DescFromEnvironment for #name {
            fn key_desc(env: &mut SalakDescContext<'_>) {
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

struct ServiceAttr {
    namespace: Option<String>,
    access: Option<u8>,
}

fn service_parse_field_attribute(attrs: Vec<Attribute>) -> ServiceAttr {
    let mut sa = ServiceAttr {
        namespace: None,
        access: None,
    };
    for attr in attrs {
        if let Ok(Meta::List(list)) = attr.parse_meta() {
            if !is_salak(&list) {
                continue;
            }
            for m in list.nested {
                if let NestedMeta::Meta(Meta::NameValue(nv)) = m {
                    match &parse_path(nv.path)[..] {
                        "namespace" => sa.namespace = Some(parse_lit(nv.lit)),
                        "access" => {
                            sa.access = Some(match &parse_lit(nv.lit)[..] {
                                "pub" => 0,
                                "pub(crate)" => 1,
                                _ => panic!("Only support \"pub\" or \"pub(crate)\""),
                            })
                        }
                        _ => panic!("Only support namespace/access"),
                    }
                } else {
                    panic!("Only support NestedMeta::Meta(Meta::NameValue)");
                }
            }
        }
    }
    sa
}

fn get_generic_type<'a>(ty: &'a Type, name: &str) -> (bool, &'a Type) {
    match ty {
        Type::Path(v) => {
            let v = v.path.segments.first().unwrap();
            if v.ident == name {
                if let PathArguments::AngleBracketed(a) = &v.arguments {
                    if let GenericArgument::Type(t) = a.args.first().unwrap() {
                        return (true, t);
                    }
                }
                panic!("Not possible")
            } else {
                (false, ty)
            }
        }
        _ => panic!("Invalid type"),
    }
}

fn service_derive_field(
    field: Field,
) -> (quote::__private::TokenStream, quote::__private::TokenStream) {
    let name = field.ident.expect("Not possible");
    let ServiceAttr { namespace, access } = service_parse_field_attribute(field.attrs);
    let namespace = namespace.unwrap_or("".to_owned());
    let (is_option, ty) = get_generic_type(&field.ty, "Option");
    let (is_arc, ty) = get_generic_type(ty, "Arc");
    if !is_arc {
        panic!("Please use Arc wrapped value.");
    }
    let fnm = quote::format_ident!("as_{}", name);
    let access = match access {
        Some(0) => quote! { pub },
        Some(1) => quote! {pub(crate)},
        _ => quote! {},
    };
    if is_option {
        (
            quote! {
                #name: factory.get_optional_resource_by_namespace::<#ty>(#namespace)?
            },
            quote! {
               #access fn #fnm(&self) -> Option<&#ty> {
                    if let Some(v) = &self.#name {
                        return Some(v.as_ref());
                    }
                    None
                }
            },
        )
    } else {
        (
            quote! {
                #name: factory.get_resource_by_namespace::<#ty>(#namespace)?
            },
            quote! {
              #access  fn #fnm(&self) -> &#ty {
                    self.#name.as_ref()
                }
            },
        )
    }
}

fn service_derive_fields(
    fields: Fields,
) -> (
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
) {
    if let Fields::Named(fields) = fields {
        let mut v = vec![];
        let mut f = vec![];
        for field in fields.named {
            let (x, y) = service_derive_field(field);
            v.push(x);
            f.push(y);
        }
        return (v, f);
    }
    panic!("Only support named body");
}

fn service_derive_struct(name: &Ident, data: DataStruct) -> quote::__private::TokenStream {
    let (field, fun) = service_derive_fields(data.fields);
    quote! {
        impl Service for #name {
            fn create(factory: &FactoryContext<'_>) -> Result<Self, PropertyError> {
                Ok(Self {
                   #(#field),*
                })
            }
        }

        impl #name {
            #(#fun)*
        }
    }
}

/// Derive [Service](https://docs.rs/salak/latest/salak/trait.Service.html).
#[proc_macro_derive(Service, attributes(salak))]
pub fn service_derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let body = match input.data {
        Data::Struct(data) => service_derive_struct(&name, data),
        _ => panic!("Only struct is supported"),
    };
    TokenStream::from(quote! {#body})
}
