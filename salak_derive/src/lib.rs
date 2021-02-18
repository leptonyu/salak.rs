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
            default: parse_attribute_args(&f.attrs, "default")
                .map(|l| match l {
                    Lit::Str(s) => Some(s.value()),
                    _ => None,
                })
                .flatten(),
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
                Ok(Self {
                    #(#ns: env.require_with_defaults::<#ts>(&format!("{}{}",&x,stringify!(#ns)), map)?),*
                })
            }

            fn default_values(prefix: &str) -> Option<std::collections::HashMap<String, Property>> {
                let mut map = std::collections::HashMap::new();
                let prefix = if prefix.is_empty() { "".to_owned() } else { format!("{}.",prefix)};
                #(map.insert(format!("{}{}", &prefix, stringify!(#mns)),Property::Str(#mds.to_owned()));)*
                Some(map)
            }
        }
    };

    gen.into()
}
