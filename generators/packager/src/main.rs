use std::path::Path;
use std::process::Command;
use std::{fs, path};

fn main() {
    // Define our source and destination paths relative to the workspace root
    let target_release_dir = Path::new("target/release");
    let packages_dir = Path::new("packages/");

    // 1. Ensure the target/release directory actually exists
    if !target_release_dir.exists() {
        eprintln!(
            "Error: 'target/release' folder not found. Did you run 'cargo build --release' first?"
        );
        std::process::exit(1);
    }

    // 2. Create the 'packages' directory if it doesn't exist
    if !packages_dir.exists() {
        println!("Creating '{}' directory...", packages_dir.display());
        fs::create_dir_all(packages_dir).expect("Failed to create packages directory");
    }

    println!(
        "Scanning for plugin libraries in {}...",
        target_release_dir.display()
    );

    // 3. Read the target/release directory
    let entries =
        fs::read_dir(target_release_dir).expect("Failed to read target/release directory");
    let mut packed_count = 0;

    for entry in entries.flatten() {
        let path = entry.path();

        // Check if the file is a dynamic library (ends with .so)
        if path.is_file()
            && path
                .extension()
                .map_or(false, |ext| ext == "so" || ext == "dll")
        {
            if let Some(file_name) = path.file_name() {
                let dest_path = packages_dir.join(file_name);

                println!("Processing: {:?}", file_name);

                // 4. Copy the file to the packages folder
                fs::copy(&path, &dest_path).expect("Failed to copy library");

                // 5. Strip the copied library to shrink its size
                let status = Command::new("strip")
                    .arg("--strip-all")
                    .arg(&dest_path)
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        println!(
                            "  -> Successfully stripped and moved to {}",
                            dest_path.display()
                        );
                        packed_count += 1;
                    }
                    _ => {
                        eprintln!(
                            "  -> Warning: Failed to strip {:?}. Is the 'strip' tool installed?",
                            file_name
                        );
                    }
                }
            }
        }
    }

    println!(
        "\n Done! Successfully packaged {} plugins into '{}/*'.",
        packed_count,
        packages_dir.display()
    );
}
