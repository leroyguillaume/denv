// IMPORTS

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, FnArg, GenericArgument, Ident, ItemTrait,
    Lifetime, Pat, PatType, Path, PathArguments, ReturnType, TraitItem, Type, TypePath,
    TypeReference,
};

// FUNCTIONS

#[proc_macro_attribute]
pub fn stub(_: TokenStream, input: TokenStream) -> TokenStream {
    let item_trait = parse_macro_input!(input as ItemTrait);
    let trait_name = &item_trait.ident;
    let stub_name = format_ident!("Stub{}", trait_name);
    let mut attrs = vec![];
    let mut with_methods = vec![];
    let mut stub_methods = vec![];
    for item in &item_trait.items {
        if let TraitItem::Method(method) = item {
            let stub_method_name = &method.sig.ident;
            let attr_name = format_ident!("{}_fn", stub_method_name);
            let with_method_name = format_ident!("with_{}", attr_name);
            let mut args = method.sig.inputs.iter();
            let receiver = match args.next() {
                Some(FnArg::Receiver(receiver)) => receiver,
                _ => panic!("The trait must be can made into  an object"),
            };
            let args: Vec<&PatType> = args
                .filter_map(|arg| match arg {
                    FnArg::Typed(arg) => Some(arg),
                    _ => None,
                })
                .collect();
            let arg_types: Vec<&Type> = args.iter().map(|arg| arg.ty.as_ref()).collect();
            let arg_names: Vec<&Pat> = args.iter().map(|arg| arg.pat.as_ref()).collect();
            let return_type = substitute_return_type(&method.sig.output);
            let fn_type = quote! {
                Fn(#(#arg_types),*) #return_type
            };
            let attr = quote! {
                pub #attr_name: Option<Box<dyn #fn_type>>
            };
            let with_method = quote! {
                pub fn #with_method_name<F: #fn_type + 'static>(&mut self, f: F) {
                    self.#attr_name = Some(Box::new(f));
                }
            };
            let stub_method = quote! {
                fn #stub_method_name(#receiver, #(#args),*) #return_type {
                    match &self.#attr_name {
                        Some(f) => f(#(#arg_names),*),
                        None => unimplemented!(stringify!(#stub_method_name)),
                    }
                }
            };
            attrs.push(attr);
            with_methods.push(with_method);
            stub_methods.push(stub_method);
        }
    }
    let expanded = quote! {
        #item_trait

        #[derive(Default)]
        pub struct #stub_name {
            #(#attrs),*
        }

        impl #stub_name {
            #(#with_methods)*
        }

        impl #trait_name for #stub_name {
            #(#stub_methods)*
        }
    };
    TokenStream::from(expanded)
}

#[inline]
fn substitute_path_type(path_ty: &TypePath) -> Type {
    let mut segments = path_ty.path.segments.clone();
    let last_seg_index = segments.len() - 1;
    let ty_args = match &segments[last_seg_index].arguments {
        PathArguments::AngleBracketed(ty_args) => {
            let mut args = ty_args.args.clone();
            let first_arg = match &args[0] {
                GenericArgument::Lifetime(lifetime) => {
                    let lifetime_str = lifetime.to_string();
                    let lifetime = if lifetime_str == "'_" {
                        Lifetime {
                            apostrophe: lifetime.apostrophe,
                            ident: Ident::new("static", lifetime.span()),
                        }
                    } else {
                        lifetime.clone()
                    };
                    GenericArgument::Lifetime(lifetime)
                }
                arg => arg.clone(),
            };
            args[0] = first_arg;
            let ty_args = AngleBracketedGenericArguments {
                args,
                colon2_token: ty_args.colon2_token,
                gt_token: ty_args.gt_token,
                lt_token: ty_args.lt_token,
            };
            PathArguments::AngleBracketed(ty_args)
        }
        ty_args => ty_args.clone(),
    };
    segments[last_seg_index].arguments = ty_args;
    let path_ty = TypePath {
        path: Path {
            leading_colon: path_ty.path.leading_colon,
            segments,
        },
        qself: path_ty.qself.clone(),
    };
    Type::Path(path_ty)
}

#[inline]
fn substitute_ref_type(ty: &TypeReference) -> Type {
    let ty = TypeReference {
        lifetime: Some(Lifetime::new("'static", Span::call_site())),
        ..ty.clone()
    };
    Type::Reference(ty)
}

#[inline]
fn substitute_return_type(ty: &ReturnType) -> ReturnType {
    match ty {
        ReturnType::Type(punc, ty) => {
            let ty = match ty.as_ref() {
                // Replace '_ lifetime by 'static
                Type::Path(ty) => substitute_path_type(ty),
                // Add 'static lifetime
                Type::Reference(ty) => substitute_ref_type(ty),
                ty => ty.clone(),
            };
            ReturnType::Type(*punc, Box::new(ty))
        }
        ty => ty.clone(),
    }
}
