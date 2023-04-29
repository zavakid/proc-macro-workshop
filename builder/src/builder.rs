use darling::FromField;
use darling::util::parse_attribute_to_meta_list;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{AngleBracketedGenericArguments, Data, DataStruct, DeriveInput, Fields, FieldsNamed, GenericArgument, PathArguments, Type, TypePath};

#[derive(Debug, Default, FromField)]
#[darling(default, attributes(builder))]
struct Opts {
    each: Option<String>,
}

struct Fd {
    name: Ident,
    ty: Type,
    opts: Opts,
}


pub struct BuilderContext {
    /// the struct name
    name: Ident,
    /// the fields named and type
    fields: Vec<Fd>,
}

impl BuilderContext {
    pub(crate) fn new(input: DeriveInput) -> Result<Self, syn::Error> {
        let name = input.ident;

        let fields = if let Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) = input.data {
            named
        } else {
            unreachable!("Unsupported struct")
        };

        let result = fields.iter().try_for_each(|f| {
            match Opts::from_field(f) {
                Ok(_) => Ok(()),
                Err(e) => {
                    let optional_attr = builder_of(f);
                    let err_str = e.to_string();
                    if err_str.contains("Unknown field:") && optional_attr.is_some(){
                        let attr = optional_attr.unwrap();
                        return match parse_attribute_to_meta_list(attr) {
                            Ok(meta_list) => {
                                Err(syn::Error::new_spanned(meta_list, "expected `builder(each = \"...\")`"))
                            }
                            Err(_) => Ok(())
                        }
                    }
                    Ok(())
                }
            }
        });

        if let Err(e) = result {
            return Err(e);
        }

        let fields = fields.into_iter().map(|f| {
            let opts = Opts::from_field(&f).unwrap_or_default();

            Fd {
                opts,
                name: f.ident.unwrap(),
                ty: f.ty,
            }
        }).collect();

        Ok(Self {
            name,
            fields,
        })
    }

    pub(crate) fn generate(&self) -> TokenStream {
        let name = &self.name;

        // builder name: {}Builder, e.g. CommandBuilder
        let builder_name = Ident::new(format!("{}Builder", name).as_str(), name.span());

        //optional field. e.g. executable: String -> executable: Option<String>,
        let optional_fields = self.gen_optional_fields();

        // methods, e.g. fn executable(mut self, v: String) -> Self { self.executable = Some(v); self }
        // Command::builder().executable("hello").args(vec![]).envs(vec![]).build();
        let methods = self.gen_methods();

        // assign builder fields back to the original struct fields
        // e.g. executable: self.executable.ok_or("executable need to set")
        let assigns = self.gen_assigns();

        let ast = quote! {
            #[derive(Debug, Default)]
            pub struct #builder_name {
                #(#optional_fields,)*
            }

            impl #builder_name {
                #(#methods)*

                pub fn build(&mut self) -> std::result::Result<#name, &'static str> {
                    std::result::Result::Ok(#name {
                        #(#assigns,)*
                    })
                }
            }

            impl #name {
                pub fn builder() -> #builder_name {
                    Default::default()
                }
            }
        };

        ast.into()
    }

    fn gen_optional_fields<'a>(&'a self) -> impl Iterator<Item=TokenStream> + 'a {
        self.fields.iter().map(|f| {
            let ty = &f.ty;
            let name = &f.name;

            if get_inner_type(ty, "Option").is_some() {
                quote! { #name : #ty }
            } else {
                quote! { #name : std::option::Option<#ty> }
            }
        })
    }

    fn gen_methods<'a>(&'a self) -> impl Iterator<Item=TokenStream> + 'a {
        self.fields.iter().map(|f| {
            let ty = &f.ty;
            let name = &f.name;

            let option = get_inner_type(ty, "Option");
            if option.is_some() {
                let inner_ty = option.unwrap();
                return quote! {
                    pub fn #name(&mut self, v: impl Into<#inner_ty>) -> &mut Self {
                        self.#name = std::option::Option::Some(v.into());
                        self
                    }
                };
            }

            let vec = get_inner_type(ty, "Vec");
            if vec.is_some() && f.opts.each.is_some() {
                let inner_ty = vec.unwrap();
                let each = Ident::new(f.opts.each.as_ref().unwrap(), f.name.span());
                return quote! {
                    pub fn #each(&mut self, v: impl Into<#inner_ty>) -> &mut Self {
                        let item = v.into();
                        let mut vec = self.#name.take().unwrap_or(vec![]);
                        vec.push(item);
                        self.#name = Some(vec);
                        self
                    }
                };
            }

            // fn executable(mut self, v: String) -> Self { self.executable = Some(v); self }
            quote! {
                pub fn #name(&mut self, v: impl Into<#ty>) -> &mut Self {
                    self.#name = Some(v.into());
                    self
                }
            }
        })
    }

    fn gen_assigns<'a>(&'a self) -> impl Iterator<Item=TokenStream> + 'a {
        self.fields.iter().map(|f| {
            let name = &f.name;

            let ty = &f.ty;
            if get_inner_type(ty, "Option").is_some() {
                quote! {
                    #name : self.#name.take()
                }
            } else if get_inner_type(ty, "Vec").is_some() {
                quote! {
                    #name : self.#name.take().unwrap_or(vec![])
                }
            } else {
                quote! {
                    #name : self.#name.take().ok_or(concat!(stringify!(#name), " need to be set!"))?
                }
            }
        })
    }
}

/// return a inner type if the input ty is equal the name
fn get_inner_type<'a>(ty: &'a Type, name: &str) -> Option<&'a Type> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            if path.segments.len() != 1 {
                return None;
            }

            let seg = path.segments.last().unwrap();
            if seg.ident != name {
                return None;
            }

            if let PathArguments::AngleBracketed(
                AngleBracketedGenericArguments { ref args, .. }
            ) = seg.arguments {

                // unwrap is safe because we are in Option context
                match args.first().unwrap() {
                    GenericArgument::Type(ty) => Some(ty),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn builder_of(f: &syn::Field) -> Option<&syn::Attribute> {
    for attr in &f.attrs {
        if attr.path().segments.len() == 1 && attr.path().segments[0].ident == "builder" {
            return Some(attr);
        }
    }
    None
}