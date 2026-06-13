use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;
use syn::{FnArg, ItemImpl, PatType, ReturnType, parse_macro_input};

#[proc_macro_attribute]
pub fn web_api(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let struct_name = &input.self_ty;

    let struct_name_str = struct_name.to_token_stream().to_string();
    let base_path = struct_name_str.to_lowercase();
    let mut filters = vec![];

    for item in &input.items {
        if let syn::ImplItem::Fn(m) = item {
            let fn_name = &m.sig.ident;
            let route_name = fn_name.to_string();

            let has_args = m.sig.inputs.len() > 1;

            let mut arg_names = vec![];
            let mut fn_arg_types = vec![];

            for input_arg in &m.sig.inputs {
                if let FnArg::Typed(PatType { pat, ty, .. }) = input_arg {
                    arg_names.push(pat);
                    fn_arg_types.push(ty.clone());
                }
            }

            let closure_arg_types: Vec<Type> = fn_arg_types
                .iter()
                .map(|ty| {
                    if let Type::Reference(type_ref) = ty.as_ref() {
                        (*type_ref.elem).clone()
                    } else {
                        *(*ty).clone()
                    }
                })
                .collect();

            let borrowed_fn_args: Vec<_> = fn_arg_types
                .iter()
                .zip(arg_names.iter())
                .map(|(ty, name)| {
                    if let Type::Reference(_) = ty.as_ref() {
                        quote! {& #name }
                    } else {
                        quote! { #name }
                    }
                })
                .collect();

            let ret_type = match &m.sig.output {
                ReturnType::Default => quote! { () },
                ReturnType::Type(_, ty) => quote! { #ty },
            };

            // Generate warp filter with smart serialization versioned path
            let filter = if has_args {
                // POST with Bitcode/JSON switching
                quote! {
                    {
                        let instance = instance.clone();
                        warp::path(#base_path)
                            .and(warp::path(#route_name))
                            .and(warp::post())
                            .and(warp::header::optional::<String>("content-type"))
                            .and(warp::header::optional::<String>("accept"))
                            .and(warp::body::bytes())
                            .and_then(move |content_type: Option<String>, accept: Option<String>, bytes: bytes::Bytes| {
                                let instance = instance.clone();
                                async move {
                                    // 1. DESERIALIZE REQUEST BODY
                                    let is_bitcode_req = content_type.as_deref() == Some("application/bitcode");
                                    let args: (#(#closure_arg_types),*) = if is_bitcode_req {
                                        // bitcode::deserialize expects a slice of bytes
                                        match bitcode::deserialize(&bytes) {
                                            Ok(val) => val,
                                            Err(_) => return Err(warp::reject::custom(BitcodeReject)),
                                        }
                                    } else {
                                        match serde_json::from_slice(&bytes) {
                                            Ok(val) => val,
                                            Err(_) => return Err(warp::reject::custom(BitcodeReject)),
                                        }
                                    };

                                    // Destructure args to names for the function call
                                    let (#(#arg_names),*) = args;
                                    let res: #ret_type = instance.#fn_name(#(#borrowed_fn_args),*);

                                    // 2. SERIALIZE RESPONSE BODY
                                    let is_bitcode_res = accept.as_deref() == Some("application/bitcode");
                                    if is_bitcode_res {
                                        match bitcode::serialize(&res) {
                                            Ok(body) => Ok(warp::reply::with_header(body, "content-type", "application/bitcode")),
                                            Err(_) => Err(warp::reject::custom(BitcodeReject)),
                                        }
                                    } else {
                                        match serde_json::to_vec(&res) {
                                            Ok(body) => Ok(warp::reply::with_header(body, "content-type", "application/json")),
                                            Err(_) => Err(warp::reject::custom(BitcodeReject)),
                                        }
                                    }
                                }
                            })
                    }
                }
            } else {
                // GET with Bitcode/JSON switching (only serialization happens here)
                quote! {
                    {
                        let instance = instance.clone();
                        warp::path(#base_path)
                            .and(warp::path(#route_name))
                            .and(warp::get())
                            .and(warp::header::optional::<String>("accept"))
                            .and_then(move |accept: Option<String>| {
                                let instance = instance.clone();
                                async move {
                                    let res: #ret_type = instance.#fn_name();

                                    let is_bitcode_res = accept.as_deref() == Some("application/bitcode");
                                    if is_bitcode_res {
                                        match bitcode::serialize(&res) {
                                            Ok(body) => Ok(warp::reply::with_header(body, "content-type", "application/bitcode")),
                                            Err(_) => Err(warp::reject::custom(BitcodeReject)),
                                        }
                                    } else {
                                        match serde_json::to_vec(&res) {
                                            Ok(body) => Ok(warp::reply::with_header(body, "content-type", "application/json")),
                                            Err(_) => Err(warp::reject::custom(BitcodeReject)),
                                        }
                                    }
                                }
                            })
                    }
                }
            };
            filters.push(filter);
        }
    }

    let combined_filters = filters
        .into_iter()
        .reduce(|acc, f| quote! { #acc.or(#f) })
        .unwrap();

    let expanded = quote! {
        #input

        // Define a custom rejection for bitcode/json error handling
        #[derive(Debug)]
        struct BitcodeReject;
        impl warp::reject::Reject for BitcodeReject {}

        impl #struct_name {
            pub fn get_filters(self) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
                use warp::Filter;
                let instance = self.clone();
                let routes = #combined_filters;
                routes.boxed()
            }
        }
    };
    TokenStream::from(expanded)
}
