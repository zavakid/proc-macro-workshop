use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{AngleBracketedGenericArguments, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, GenericArgument, PathArguments, Type, TypePath};
use syn::punctuated::Punctuated;
use syn::token::Comma;

pub struct BuilderContext {
    /// the struct name
    name: Ident,
    /// the fields named and type
    fields: Punctuated<Field, Comma>,
}

impl BuilderContext {
    pub(crate) fn new(input: DeriveInput) -> Self {
        let name = input.ident;

        let fields = if let Data::Struct(DataStruct { fields: Fields::Named(FieldsNamed { named, .. }), .. }) = input.data {
            named
        } else {
            unreachable!("Unsupported struct")
        };

        Self {
            name,
            fields,
        }
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

                pub fn build(&mut self) -> Result<#name, &'static str> {
                    Ok(#name{
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
            let name = &f.ident;

            if get_option_arg(ty).is_some() {
                quote! { #name : #ty }
            } else {
                quote! { #name : std::option::Option<#ty> }
            }
        })
    }

    fn gen_methods<'a>(&'a self) -> impl Iterator<Item=TokenStream> + 'a {
        self.fields.iter().map(|f| {
            let ty = &f.ty;
            let name = &f.ident;

            let option = get_option_arg(ty);

            if option.is_some() {
                let inner_ty = option.unwrap();
                quote! {
                    pub fn #name(&mut self, v: impl Into<#inner_ty>) -> &mut Self {
                        self.#name = Some(v.into());
                        self
                    }
                }
            } else {
                // fn executable(mut self, v: String) -> Self { self.executable = Some(v); self }
                quote! {
                    pub fn #name(&mut self, v: impl Into<#ty>) -> &mut Self {
                        self.#name = Some(v.into());
                        self
                    }
                }
            }
        })
    }

    fn gen_assigns<'a>(&'a self) -> impl Iterator<Item=TokenStream> + 'a {
        self.fields.iter().map(|f| {
            let name = &f.ident;

            let ty = &f.ty;
            if get_option_arg(ty).is_some() {
                quote! {
                    #name : self.#name.take()
                }
            } else {
                quote! {
                    #name : self.#name.take().ok_or(concat!(stringify!(#name), " need to be set!"))?
                }
            }
        })
    }
}

fn get_option_arg(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            if path.segments.len() != 1 {
                return None;
            }

            let seg = path.segments.last().unwrap();
            if seg.ident != "Option" {
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