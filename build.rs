use quote::{ToTokens, quote};
use std::env;
use std::fs::{File, read_to_string};
use std::io::{self, Write};
use syn::Visibility;
use syn::{ImplItem, ItemImpl};

fn generate_client_code(api_file: &str) -> io::Result<String> {
    // Read the file content and handle errors
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

    // Generate client methods for each function in impl blocks
    for item in syntax_tree.items.iter() {
        if let syn::Item::Impl(ItemImpl {
            attrs,
            items,
            self_ty,
            ..
        }) = item
        {
            // Check if the impl block has the `web_api` macro
            if !has_web_api_macro(attrs) {
                continue; // Skip this impl block if `web_api` is not present
            }

            let struct_name_str = &self_ty.to_token_stream().to_string();
            let base_path = struct_name_str.to_lowercase();

            for item in items {
                if let ImplItem::Fn(fn_item) = item {
                    // Only generate docs if we are public
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

                    // Extract argument names and types
                    for input in &fn_item.sig.inputs {
                        if let syn::FnArg::Typed(pat_type) = input {
                            arg_names.push(&pat_type.pat);
                            arg_types.push(&pat_type.ty);
                        }
                    }

                    // Handle return types
                    let ret_type = match &fn_item.sig.output {
                        syn::ReturnType::Default => quote! { () },
                        syn::ReturnType::Type(_, ty) => quote! { #ty },
                    };

                    let doc_string = documentation.join("\n\n");

                    // Generate client function with arguments
                    let client_fn = if has_args {
                        quote! {
                            #[doc = #doc_string]
                            pub fn #fn_name(&self, #(#arg_names: #arg_types),*) -> Result<#ret_type, ureq::Error> {
                                let url = format!("{}/{}/{}", self.base_url, #base_path, #route_name);
                                let res = ureq::post(url)
                                    .send_json(&(#(#arg_names),*))?
                                    .body_mut()
                                    .read_json::<#ret_type>()?;
                                Ok(res)
                            }
                        }
                    } else {
                        // Generate client function without arguments
                        quote! {
                            #[doc = #doc_string]
                            pub fn #fn_name(&self) -> Result<#ret_type, ureq::Error> {
                                let url = format!("{}/{}/{}", self.base_url, #base_path, #route_name);
                                let res = ureq::get(url)
                                    .call()?
                                    .body_mut()
                                    .read_json::<#ret_type>()?;
                                Ok(res)
                            }
                        }
                    };

                    client_functions.push(client_fn);
                }
            }
        }
    }

    // Combine client code with generated functions
    let client_code = quote! {
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

    Ok(client_code.to_string())
}

fn has_web_api_macro(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        // Check if the attribute path is "web_api"
        if attr.path().is_ident("web_api") {
            return true;
        }
    }
    false
}
fn write_client_file(client_code: &str) -> io::Result<()> {
    println!(
        "Current working directory: {}",
        env::current_dir().unwrap().display()
    );

    if std::path::Path::new("src/generated/client_api.rs").exists() {
        std::fs::remove_file("src/generated/client_api.rs");
    }

    // Write the formatted code to client.rs
    let mut file = File::create("src/generated/client_api.rs").map_err(|e| {
        eprintln!("Error creating client.rs: {}", e);
        e
    })?;
    file.write_all(client_code.as_bytes()).map_err(|e| {
        eprintln!("Error writing to client.rs: {}", e);
        e
    })?;

    println!("client.rs has been written successfully.");
    Ok(())
}

fn main() {
    {
        let file_path = "./src/database/database.rs";
        dbg!(&file_path);
        if let Ok(ref code) = generate_client_code(file_path) {
            let _ = write_client_file(code);
        }
    }

    /*let plugins_path = "./Plugins";
    let onesec = Duration::from_secs(100);
    let pp = Path::new("soup.txt");
    let mut file = fs::File::create(pp).unwrap();
    for entry in walkdir::WalkDir::new(plugins_path) {
        file.write(&format!("{:?}", entry.unwrap()).into_bytes())
            .unwrap();
        println!("a");
    }
    file.flush();*/
}
