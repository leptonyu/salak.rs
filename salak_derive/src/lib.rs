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

fn is_salak(list: &MetaList) -> bool {
    if let Some(v) = list.path.segments.iter().next() {
        return v.ident == "salak";
    }
    false
}

fn parse_field_attribute(
    attrs: Vec<Attribute>,
    get_val: quote::__private::TokenStream,
    name: &mut Ident,
    optional: bool,
) -> (
    quote::__private::TokenStream,
    Option<quote::__private::TokenStream>,
    quote::__private::TokenStream,
) {
    let mut def = None;
    let mut rename = None;
    let mut required = None;
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
                        "required" => match &parse_lit(nv.lit)[..] {
                            "true" => required = Some(true),
                            "false" => required = Some(false),
                            _ => panic!("required only support bool value"),
                        },
                        _ => panic!("Only support default/name/required"),
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

    let required = if let Some(v) = required { v } else { !optional };
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
            quote! {
                (stringify!(#name).to_owned(), false, Some(Property::Str(#def.to_owned())))
            },
        ),
        _ => (
            get_val,
            None,
            quote! {
                (stringify!(#name).to_owned(), #required, None)
            },
        ),
    }
}

fn derive_field(
    field: Field,
    optional: bool,
) -> (
    quote::__private::TokenStream,
    quote::__private::TokenStream,
    Option<quote::__private::TokenStream>,
    quote::__private::TokenStream,
) {
    let name = field.ident.expect("Not possible");
    let ty = field.ty;
    let temp_name = quote::format_ident!("__{}", name);
    let get_value = quote! {
      env.require::<Option<Property>>(&#temp_name)?
    };
    let mut rename = name.clone();
    let (get_value, def, def_all) =
        parse_field_attribute(field.attrs, get_value, &mut rename, optional);
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
        def_all,
    )
}

fn is_primitive(ty: &Type) -> bool {
    lazy_static::lazy_static! {
    static ref PRIMITIVE: std::collections::HashSet<String>
    = vec!["String", "u8", "u16", "u32", "u64", "u128", "usize"
    , "i8", "i16", "i32", "i64", "i128", "isize", "f64", "f32", "bool"
    , "Duration", "DateTime", "LevelFilter", "Level"]
        .into_iter().map(|a|a.to_owned()).collect();
    }
    if let Type::Path(x) = &ty {
        if let Some(ident) = x.path.segments.iter().next() {
            if is_container(&ident.ident) {
                if let PathArguments::AngleBracketed(params) =
                    &x.path.segments.iter().next().unwrap().arguments
                {
                    if let Some(GenericArgument::Type(ty)) = params.args.iter().next() {
                        return is_primitive(ty);
                    }
                }
            } else if PRIMITIVE.contains(&format!("{}", ident.ident)) {
                return true;
            }
        }
    }
    false
}

fn is_container(ident: &Ident) -> bool {
    ident == "Option" || ident == "Vec" || ident == "HashMap" || ident == "HashSet"
}

fn is_option(ty: &Type) -> bool {
    if let Type::Path(x) = &ty {
        if let Some(ident) = x.path.segments.iter().next() {
            return is_container(&ident.ident);
        }
    }
    false
}

fn derive_fields(
    fields: Fields,
) -> (
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
    Vec<quote::__private::TokenStream>,
    quote::__private::TokenStream,
    quote::__private::TokenStream,
) {
    if let Fields::Named(fields) = fields {
        let mut k = vec![];
        let mut v = vec![];
        // Only field with defaults.
        let mut d = vec![];
        let mut n = vec![];
        // All fields.
        let mut dl = vec![];
        let mut nl = vec![];
        for field in fields.named {
            let ty = field.ty.clone();
            let name = field.ident.clone();
            if !is_primitive(&ty) {
                n.push(quote! {
                    (stringify!(#name), <#ty>::load_default())
                });
            }
            let (temp_name, get_env, def, def_all) = derive_field(field, is_option(&ty));
            k.push(temp_name);
            v.push(get_env);
            if let Some(def) = def {
                d.push(def);
            }

            if !is_primitive(&ty) {
                nl.push(quote! {
                    (stringify!(#name), <#ty>::load_keys())
                });
            } else {
                dl.push(def_all);
            }
        }
        let n = if n.is_empty() {
            quote! {}
        } else {
            quote! {
                for (p, vs) in vec![#(#n),*] {
                    for (n, s) in vs {
                        v.push((format!("{}.{}", p, n), s));
                    }
                }
            }
        };
        let nl = if nl.is_empty() {
            quote! {}
        } else {
            quote! {
                for (p, vs) in vec![#(#nl),*] {
                    for (n, o, s) in vs {
                        v.push((format!("{}.{}", p, n), o, s));
                    }
                }
            }
        };
        (k, v, d, dl, n, nl)
    } else {
        panic!("Only support named body");
    }
}

fn derive_struct(
    data: DataStruct,
) -> (quote::__private::TokenStream, quote::__private::TokenStream) {
    let (expr, field, defs, def_all, ns, nl) = derive_fields(data.fields);
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
                #ns
                v
            }
            fn load_keys() -> Vec<(String, bool, Option<Property>)> {
                let mut v = vec![];
                #(v.push(#def_all);)*
                #nl
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
    let (def, _, _) =
        parse_field_attribute(attrs, quote! { property }, &mut type_name.clone(), false);
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
            return match &std::convert::TryInto::<String>::try_into(p)?[..] {
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
