// IMPORTS

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, FnArg, ItemTrait, Pat, PatType, TraitItem, Type};

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
            let return_type = &method.sig.output;
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
                        None => unimplemented!(),
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
