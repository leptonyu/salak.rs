use proc_macro::TokenStream;
use syn::*;

struct FieldAttr {
    name: Ident,
    ty: Type,
    use_raw: bool,
    default: Option<String>,
}

impl From<&Field> for FieldAttr {
    fn from(f: &Field) -> FieldAttr {
        let mut fa = FieldAttr {
            name: f.ident.clone().unwrap(),
            ty: f.ty.clone(),
            use_raw: false,
            default: None,
        };

        parse_attribute_args(&f.attrs, &mut fa);

        fa
    }
}

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

fn parse_attribute_args(attrs: &Vec<Attribute>, fa: &mut FieldAttr) {
    for attr in attrs {
        if let Some(v) = attr.parse_meta().ok() {
            match v {
                Meta::List(list) => {
                    for m in list.nested {
                        if let NestedMeta::Meta(meta) = m {
                            match meta {
                                Meta::Path(path) => {
                                    if parse_path(path) == "disable_placeholder" {
                                        fa.use_raw = true;
                                    } else {
                                        panic!("Only support disable_placeholder");
                                    }
                                }
                                Meta::NameValue(nv) => match &parse_path(nv.path)[..] {
                                    "disable_placeholder" => fa.use_raw = parse_lit_bool(nv.lit),
                                    "default" => fa.default = Some(parse_lit(nv.lit)),
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
}

#[proc_macro_derive(FromEnvironment, attributes(salak))]
pub fn from_env_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = ast.ident;
    let data = match ast.data {
        Data::Struct(d) => d,
        _ => panic!("FromEnvironment only support struct"),
    };

    let mut ns = vec![];
    let mut ts = vec![];
    let mut rs = vec![];
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
        rs.push(fa.use_raw);
        if let Some(d) = fa.default {
            mns.push(fa.name);
            mds.push(d);
        }
    }
    let gen = quote::quote! {
        impl FromEnvironment for #name {
            fn from_env(n: &str, _: Option<Property>,
                env: &impl Environment,
                _: bool,
                mut_option: &mut EnvironmentOption,
            ) -> Result<Self, PropertyError>{
                let n = n.to_prefix();
                #(mut_option.insert(format!("{}{}",n,stringify!(#mns)),#mds);)*
                Ok(Self {
                    #(#ns: env.require_with_options::<#ts>(&format!("{}{}",n,stringify!(#ns)), #rs, mut_option)?),*
                })
            }
        }
    };

    gen.into()
}
