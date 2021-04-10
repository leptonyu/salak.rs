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
    }
    None
}

fn parse_field_attribute(
    attrs: Vec<Attribute>,
    get_val: quote::__private::TokenStream,
    name: &mut Ident,
) -> (
    quote::__private::TokenStream,
    Option<quote::__private::TokenStream>,
) {
    let mut def = None;
    let mut rename = None;
    for attr in attrs {
        if let Ok(v) = attr.parse_meta() {
            match v {
                Meta::List(list) => {
                    for m in list.nested {
                        if let NestedMeta::Meta(meta) = m {
                            match meta {
                                Meta::NameValue(nv) => match &parse_path(nv.path)[..] {
                                    "default" => def = Some(parse_lit(nv.lit)),
                                    "name" => rename = Some(parse_lit(nv.lit)),
                                    _ => panic!("Only support default/name"),
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
    if let Some(rename) = rename {
        *name = quote::format_ident!("{}", rename);
    }

    match def {
        Some(def) => (
            quote! {
                match #get_val {
                    None => env.resolve_placeholder(#def.to_string())?,
                    v    => v,
                }
            },
            Some(quote! {
                (stringify!(#name).to_owned(), Property::Str(#def.to_owned()))
            }),
        ),
        _ => (get_val, None),
    }
}

fn derive_field(
    field: Field,
) -> (
    quote::__private::TokenStream,
    quote::__private::TokenStream,
    Option<quote::__private::TokenStream>,
) {
    let name = field.ident.expect("Not possible");
    let ty = field.ty;
    let temp_name = quote::format_ident!("__{}", name);
    let get_value = quote! {
      env.require::<Option<Property>>(&#temp_name)?
    };
    let mut rename = name.clone();
    let (get_value, def) = parse_field_attribute(field.attrs, get_value, &mut rename);
    (
        quote! {
            let #temp_name = format!("{}{}", name ,stringify!(#rename));
        },
        quote! {
            #name: <#ty>::from_env(&#temp_name,
                #get_value,
                env)?
        },
        def,
    )
}

fn derive_fields(
    fields: Fields,
) -> (
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
) {
    if let Fields::Named(fields) = fields {
        let mut k = vec![];
        let mut v = vec![];
        let mut d = vec![];
        let mut n = vec![];
        for field in fields.named {
            let ty = field.ty.clone();
            let name = field.ident.clone();
            n.push(quote! {
                (stringify!(#name), <#ty>::load_default())
            });
            let (temp_name, get_env, def) = derive_field(field);
            k.push(temp_name);
            v.push(get_env);
            if let Some(def) = def {
                d.push(def);
            }
        }
        (k, v, d, n)
    } else {
        panic!("Only support named body");
    }
}

fn derive_struct(
    data: DataStruct,
) -> (quote::__private::TokenStream, quote::__private::TokenStream) {
    let (expr, field, defs, ns) = derive_fields(data.fields);
    (
        quote! {
            let name = name.to_prefix();
            #(#expr)*
            Ok(Self {
                #(#field),*
            })
        },
        quote! {
            fn load_default() -> Vec<(String, Property)> {
                let mut v = vec![];
                #(v.push(#defs);)*
                for (p, vs) in vec![#(#ns),*] {
                    for (n, s) in vs {
                        v.push((format!("{}.{}", p, n), s));
                    }
                }
                v
            }
        },
    )
}

fn derive_enum(
    type_name: Ident,
    attrs: Vec<Attribute>,
    data: DataEnum,
) -> quote::__private::TokenStream {
    let (def, _) = parse_field_attribute(attrs, quote! { property }, &mut type_name.clone());
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
        if let Some(p) = #def {
            return match &String::from_property(p)?[..] {
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
    let (head, (body, default)) = match input.data {
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
        Data::Enum(d) => (
            quote! {},
            (derive_enum(name.clone(), input.attrs, d), quote! {}),
        ),
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
            #default
        }
        impl AutoDeriveFromEnvironment for #name {}
        #head
    })
}
