
use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{parse_macro_input, ItemImpl, FnArg, PatType, ReturnType};

use syn::Type;





#[proc_macro_attribute]
pub fn web_api(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let struct_name = &input.self_ty;
    // Use the struct name as the base path for versioning
    let struct_name_str = struct_name.to_token_stream().to_string();
    let base_path = struct_name_str.to_lowercase();
    let mut filters = vec![];

    // Loop through the items in the implementation block
    for item in &input.items {
        if let syn::ImplItem::Fn(m) = item {
            let fn_name = &m.sig.ident;
            let route_name = fn_name.to_string();
            
            // Detect if function has arguments (skip &self)
            let has_args = m.sig.inputs.len() > 1;

            // Collect argument names/types for the function signature
            let mut arg_names = vec![];
            let mut fn_arg_types = vec![];

            for input_arg in &m.sig.inputs {
                if let FnArg::Typed(PatType { pat, ty, .. }) = input_arg {
                    arg_names.push(pat);
                    fn_arg_types.push(ty.clone());
                }
            }

            // Determine the *owned* types for the warp map closure argument
            let closure_arg_types: Vec<Type> = fn_arg_types.iter().map(|ty| {
                if let Type::Reference(type_ref) = ty.as_ref() {
                    // Convert '&T' to 'T' (e.g., '&usize' to 'usize')
                    (*type_ref.elem).clone()
                } else {
                    *(*ty).clone()
                }
            }).collect();
            
            // The function call inside the map needs to borrow the owned data if the function expects a reference.
            let borrowed_fn_args: Vec<_> = fn_arg_types.iter().zip(arg_names.iter()).map(|(ty, name)| {
                if let Type::Reference(_) = ty.as_ref() {
                    quote! {& #name } // Use the identifier as a reference
                } else {
                    quote! { #name } // Use the identifier directly (owned)
                }
            }).collect();


            // Return type
            let ret_type = match &m.sig.output {
                ReturnType::Default => quote! { () },
                ReturnType::Type(_, ty) => quote! { #ty },
            };

            // Generate warp filter with versioned path
            let filter = if has_args {
                // POST with JSON body
                quote! {
                    {
                        let instance = instance.clone();
                        warp::path(#base_path)
                            .and(warp::path(#route_name))
                            .and(warp::post())
                            .and(warp::body::json())
                            // map receives owned types (usize, String)
                            .map(move |(#(#arg_names),*): (#(#closure_arg_types),*)| {
                                // function call uses the appropriate borrowed/owned form
                                let res: #ret_type = instance.#fn_name(#(#borrowed_fn_args),*);
                                warp::reply::json(&res)
                            })
                    }
                }
            } else {
                // GET (no body)
                quote! {
                    {
                        let instance = instance.clone();
                        warp::path(#base_path)
                            .and(warp::path(#route_name))
                            .and(warp::get())
                            .map(move || {
                                let res: #ret_type = instance.#fn_name();
                                warp::reply::json(&res)
                            })
                    }
                }
            };
            filters.push(filter);
        }
    }

    // Combine filters (i.e., combine all routes for the struct)
    let combined_filters = filters.into_iter().reduce(|acc, f| quote! { #acc.or(#f) }).unwrap();

    // Expand the code: include struct methods but do not affect its data
    let expanded = quote! {
        #input

        // This separate impl block holds the generated get_filters method
        impl #struct_name {
            pub fn get_filters(self) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
                use warp::Filter;
                let instance = self.clone();
                let routes = #combined_filters;
                routes
            }
        }
    };
    TokenStream::from(expanded)
}


