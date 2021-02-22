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
        Lit::Byte(_) => panic!("Salak not support byte"),
        Lit::Verbatim(_) => panic!("Salak not support Verbatim"),
    }
}

fn parse_lit_bool(lit: Lit) -> bool {
    match lit {
        Lit::Bool(b) => b.value,
        _ => panic!("Please use bool value"),
    }
}

fn parse_attribute_prefix(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if let Ok(v) = attr.parse_meta() {
            match v {
                Meta::List(list) => {
                    for m in list.nested {
                        if let NestedMeta::Meta(meta) = m {
                            match meta {
                                Meta::NameValue(nv) => {
                                    if parse_path(nv.path) == "prefix" {
                                        match nv.lit {
                                            Lit::Str(s) => return Some(s.value()),
                                            _ => panic!("Only support string"),
                                        }
                                    } else {
                                        panic!("Only support prefix");
                                    }
                                }
                                _ => panic!("Only support prefix=\"xxx\""),
                            }
                        }
                    }
                }
                _ => panic!("Only support #[salak(prefix=\"hello.world\")]"),
            }
        }
    }
    None
}

fn parse_field_attribute(attrs: Vec<Attribute>) -> (bool, Option<String>) {
    let mut disable_placeholder = false;
    let mut def = None;
    for attr in attrs {
        if let Ok(v) = attr.parse_meta() {
            match v {
                Meta::List(list) => {
                    for m in list.nested {
                        if let NestedMeta::Meta(meta) = m {
                            match meta {
                                Meta::Path(path) => {
                                    if parse_path(path) == "disable_placeholder" {
                                        disable_placeholder = true;
                                    } else {
                                        panic!("Only support disable_placeholder");
                                    }
                                }
                                Meta::NameValue(nv) => match &parse_path(nv.path)[..] {
                                    "disable_placeholder" => {
                                        disable_placeholder = parse_lit_bool(nv.lit)
                                    }
                                    "default" => def = Some(parse_lit(nv.lit)),
                                    _ => panic!("Only support disable_placeholder/default"),
                                },
                                _ => panic!("Not support Meta::List"),
                            }
                        }
                    }
                }
                _ => panic!("Not support Path/NameValue"),
            }
        }
    }

    (disable_placeholder, def)
}

fn derive_field(field: Field) -> (quote::__private::TokenStream, quote::__private::TokenStream) {
    let name = field.ident.expect("Not possible");
    let ty = field.ty;
    let (_, def) = parse_field_attribute(field.attrs);
    let temp_name = quote::format_ident!("__{}", name);
    let property = match def {
        Some(def) => quote! {
            .or(Some(#def.to_string()))
        },
        _ => quote! {},
    };
    (
        quote! {
            let #temp_name = format!("{}{}", name ,stringify!(#name));
        },
        quote! {
            #name: <#ty>::from_env(&#temp_name,
                env.require::<Option<String>>(&#temp_name)?#property.map(|p|Property::Str(p)),
                env)?
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
        let mut k = vec![];
        let mut v = vec![];
        for field in fields.named {
            let (a, b) = derive_field(field);
            k.push(a);
            v.push(b);
        }
        (k, v)
    } else {
        panic!("Only support named body");
    }
}

fn derive_struct(data: DataStruct) -> quote::__private::TokenStream {
    let (expr, field) = derive_fields(data.fields);
    quote! {
        let name = name.to_prefix();
        #(#expr)*
        Ok(Self {
            #(#field),*
        })
    }
}

fn derive_enum(
    type_name: Ident,
    attrs: Vec<Attribute>,
    data: DataEnum,
) -> quote::__private::TokenStream {
    let (_, def) = parse_field_attribute(attrs);
    let def = if let Some(def) = def {
        quote! {
            .or(Some(#def))
        }
    } else {
        quote! {}
    };
    let mut vs = vec![];
    for variant in data.variants {
        let name = variant.ident;
        let body = match variant.fields {
            Fields::Unit => {
                quote! {
                    stringify!(#name) => Ok(#type_name::#name),
                }
            }
            _ => panic!("Enum only support no field pattern."),
        };
        vs.push(body);
    }
    quote! {
        if let Some(def) = env.require::<Option<String>>(name)?#def {
            return match &def[..] {
                #(#vs)*
                v => Err(PropertyError::ParseFail(format!("Enum value invalid {}", v))),
            };
        }
        Err(PropertyError::NotFound(name.to_owned()))
    }
}

#[proc_macro_derive(FromEnvironment, attributes(salak))]
pub fn from_env_derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (head, body) = match input.data {
        Data::Struct(d) => (
            if let Some(prefix) = parse_attribute_prefix(&input.attrs) {
                quote! {
                        impl DefaultSourceFromEnvironment for #name {
                        fn prefix() -> &'static str {
                            #prefix
                        }
                    }
                }
            } else {
                quote! {}
            },
            derive_struct(d),
        ),
        Data::Enum(d) => (quote! {}, derive_enum(name.clone(), input.attrs, d)),
        _ => panic!("union is not supported"),
    };

    TokenStream::from(quote! {
        impl FromEnvironment for #name {
            fn from_env(
                name: &str,
                property: Option<Property>,
                env: &impl Environment,
            ) -> Result<Self, PropertyError> {
                #body
            }
        }
        impl AutoDeriveFromEnvironment for #name {}
        #head
    })
}
