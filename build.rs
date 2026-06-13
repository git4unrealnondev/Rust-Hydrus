use quote::{ToTokens, quote};
use std::fs;
use std::fs::read_to_string;
use std::io::{self, Write};
use std::path::Path;
use syn::Visibility;
use syn::parse_file;
use syn::{ImplItem, ItemImpl};

fn generate_client_code(api_file: &str) -> io::Result<String> {
    let content = read_to_string(api_file).map_err(|e| {
        eprintln!("Error reading file '{}': {}", api_file, e);
        e
    })?;

    println!("Successfully read content from '{}'", api_file);

    let syntax_tree = match syn::parse_file(&content) {
        Ok(tree) => {
            println!("Successfully parsed file.");
            tree
        }
        Err(e) => {
            eprintln!("Error parsing the file '{}': {}", api_file, e);
            return Err(io::Error::other("Failed to parse file"));
        }
    };

    let mut client_functions = vec![];

    for item in syntax_tree.items.iter() {
        if let syn::Item::Impl(ItemImpl {
            attrs,
            items,
            self_ty,
            ..
        }) = item
        {
            if !has_web_api_macro(attrs) {
                continue;
            }

            let struct_name_str = &self_ty.to_token_stream().to_string();
            let base_path = struct_name_str.to_lowercase();

            for item in items {
                if let ImplItem::Fn(fn_item) = item {
                    if fn_item.vis == Visibility::Inherited {
                        continue;
                    }
                    let mut documentation = Vec::new();
                    for attr in fn_item.attrs.iter() {
                        if let Ok(name_value) = attr.meta.require_name_value()
                            && let syn::Expr::Lit(expr_lit) = &name_value.value
                            && let syn::Lit::Str(expr_string) = &expr_lit.lit
                            && !expr_string.value().is_empty()
                        {
                            documentation.push(expr_string.value());
                        }
                    }
                    let fn_name = &fn_item.sig.ident;
                    let route_name = fn_name.to_string();
                    let has_args = fn_item.sig.inputs.len() > 1;
                    let mut arg_names = vec![];
                    let mut arg_types = vec![];

                    for input in &fn_item.sig.inputs {
                        if let syn::FnArg::Typed(pat_type) = input {
                            arg_names.push(&pat_type.pat);
                            arg_types.push(&pat_type.ty);
                        }
                    }

                    let ret_type = match &fn_item.sig.output {
                        syn::ReturnType::Default => quote! { () },
                        syn::ReturnType::Type(_, ty) => quote! { #ty },
                    };

                    let doc_string = documentation.join("\n\n");

                    // Generate client functions with properly boxed ureq::Error::Other variants
                    let client_fn = if has_args {
                        quote! {
                            #[doc = #doc_string]
                            pub fn #fn_name(&self, #(#arg_names: #arg_types),*) -> Result<#ret_type, ureq::Error> {
                                let url = format!("{}/{}/{}", self.base_url, #base_path, #route_name);

                                // Serialize arguments into a tuple using bitcode
                                let payload = bitcode::serialize(&(#(#arg_names),*))
                                    .map_err(|e| ureq::Error::Other(Box::new(e)))?;

                                let response_bytes = ureq::post(url)
                                    .header("content-type", "application/bitcode")
                                    .header("accept", "application/bitcode")
                                    .send(payload)?
                                    .into_body()
                                    .read_to_vec()?;

                                let res: #ret_type = bitcode::deserialize(&response_bytes)
                                    .map_err(|e| ureq::Error::Other(Box::new(e)))?;

                                Ok(res)
                            }
                        }
                    } else {
                        quote! {
                            #[doc = #doc_string]
                            pub fn #fn_name(&self) -> Result<#ret_type, ureq::Error> {
                                let url = format!("{}/{}/{}", self.base_url, #base_path, #route_name);

                                let response_bytes = ureq::get(url)
                                    .header("accept", "application/bitcode")
                                    .call()?
                                    .into_body()
                                    .read_to_vec()?;

                                let res: #ret_type = bitcode::deserialize(&response_bytes)
                                    .map_err(|e| ureq::Error::Other(Box::new(e)))?;

                                Ok(res)
                            }
                        }
                    };

                    client_functions.push(client_fn);
                }
            }
        }
    }

    let client_code = quote! {
        use std::collections::HashMap;
        use std::collections::HashSet;
        use std::path::PathBuf;
        use std::collections::BTreeMap;
        use crate::sharedtypes;

        #[derive(Debug)]
        pub struct RustHydrusApiClient {
            pub base_url: String,
        }

        #[allow(dead_code)]
        impl RustHydrusApiClient {
            pub fn new<S: Into<String>>(base_url: S) -> Self {
                let base_url_str = base_url.into();
                let base_url_temp = if !base_url_str.starts_with("http") {
                    format!("http://{}", base_url_str)
                } else {
                    base_url_str
                };
                RustHydrusApiClient { base_url: base_url_temp }
            }

            #(#client_functions)*
        }
    };

    let syntax_tree = parse_file(&client_code.to_string()).expect("Unable to parse generated code");
    let formatted_code = prettyplease::unparse(&syntax_tree);
    Ok(formatted_code.to_string())
}

fn has_web_api_macro(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("web_api") {
            return true;
        }
    }
    false
}

fn write_client_file_if_changed(client_code: &str) -> io::Result<()> {
    let client_path = Path::new("generated/client_api.rs");

    if client_path.exists() {
        let existing_code = fs::read_to_string(client_path)?;
        if existing_code == client_code {
            println!("cargo:warning=Client file unchanged, skipping write");
            return Ok(());
        }
    }

    if let Some(parent) = client_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(client_path)?;
    file.write_all(client_code.as_bytes())?;
    println!("cargo:warning=Client file updated");
    Ok(())
}

fn main() {
    let file_path = "./src/database/public_calls.rs";
    dbg!(&file_path);
    if let Ok(ref code) = generate_client_code(file_path) {
        let _ = write_client_file_if_changed(code);
    }
}
