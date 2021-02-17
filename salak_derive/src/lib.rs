use proc_macro::TokenStream;
use syn::*;

struct FieldAttr {
    name: Ident,
    ty: Type,
    default: Option<Lit>,
}

impl From<&Field> for FieldAttr {
    fn from(f: &Field) -> FieldAttr {
        FieldAttr {
            name: f.ident.clone().unwrap(),
            ty: f.ty.clone(),
            default: parse_attribute_args(&f.attrs, "default"),
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
    let prefix = parse_attribute_args(&ast.attrs, "prefix")
        .map(|x| match x {
            Lit::Str(s) => {
                let mut s = s.value();
                s.push_str(".");
                Some(s)
            }
            _ => None,
        })
        .flatten()
        .unwrap_or("".to_owned());

    let mut ns = vec![];
    let mut ds = vec![];
    let mut ts = vec![];

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
        ns.push(fa.name);
        ts.push(fa.ty);
        ds.push(fa.default);
    }
    let gen = quote::quote! {
        impl FromEnvironment for #name {
            fn from_env(env: &impl Environment) -> Result<Self, PropertyError>{
                Ok(Self {
                    #(#ns: env.require::<#ts>(&format!("{}{}", #prefix, stringify!(#ns)))?),*
                })
            }
        }
    };

    gen.into()
}
