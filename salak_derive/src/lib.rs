use proc_macro::TokenStream;
use syn::*;

struct FieldAttr {
    name: Ident,
    ty: Type,
    default: Option<String>,
}

impl From<&Field> for FieldAttr {
    fn from(f: &Field) -> FieldAttr {
        FieldAttr {
            name: f.ident.clone().unwrap(),
            ty: f.ty.clone(),
            default: parse_attribute_args(&f.attrs, "default").map(|l| match l {
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
            }),
        }
    }
}

fn parse_attribute_args(attrs: &Vec<Attribute>, target: &str) -> Option<Lit> {
    attrs
        .first()
        .map(|attr| attr.parse_meta().ok())
        .flatten()
        .map(|meta| match meta {
            Meta::List(list) => {
                for arg in list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(n)) = arg {
                        let name = n.path.segments.first().unwrap().ident.to_string();
                        if name == target {
                            return Some(n.lit);
                        }
                    }
                }
                None
            }
            _ => panic!("Couldn't parse attribute arguments"),
        })
        .flatten()
}

#[proc_macro_derive(FromEnvironment, attributes(field))]
pub fn from_env_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = ast.ident;
    let data = match ast.data {
        Data::Struct(d) => d,
        _ => panic!("FromEnvironment only support struct"),
    };

    let mut ns = vec![];
    let mut ts = vec![];
    let mut mns = vec![];
    let mut mds = vec![];

    for fa in match data.fields {
        Fields::Named(FieldsNamed {
            brace_token: _,
            named,
        }) => named,
        _ => panic!("Struct fields must be named"),
    }
    .iter()
    .map(|x| x.into())
    .collect::<Vec<FieldAttr>>()
    {
        ns.push(fa.name.clone());
        ts.push(fa.ty);
        if let Some(d) = fa.default {
            mns.push(fa.name);
            mds.push(d);
        }
    }
    let gen = quote::quote! {
        impl FromEnvironment for #name {
            fn from_env(n: &str, _: Option<Property>, env: &impl Environment, map: &mut map::MapPropertySource) -> Result<Self, PropertyError>{
                let x = if n.is_empty() { "".to_owned() } else { format!("{}.", n) };
                let mut dmap = std::collections::HashMap::new();
                #(dmap.insert(stringify!(#mns).to_owned(),Property::Str(#mds.to_owned()));)*
                map.insert(n, dmap);
                Ok(Self {
                    #(#ns: env.require_with_defaults::<#ts>(&format!("{}{}",&x,stringify!(#ns)), map)?),*
                })
            }
        }
    };

    gen.into()
}
