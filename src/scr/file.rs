use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};

use crate::database::{enclave, Main};
use crate::download::hash_file;
use crate::{logging, sharedtypes, Arc, RwLock};
use std::fs::File;
use std::io::{self, BufRead};

/// Returns OK or Err if file size is eq to inint.
pub fn size_eq(input: String, inint: u64) -> std::io::Result<()> {
    let size = fs::metadata(input)?;
    if inint == size.len() {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
    // assert_eq!(inint, size.len());
}

/// Removes a file from the folder.
pub fn remove_file(input: String) -> std::io::Result<()> {
    fs::remove_file(input)?;
    Ok(())
}

/// Make Folder
pub fn folder_make(location: &String) {
    if let Err(err) = std::fs::create_dir_all(location) {
        logging::error_log(&format!("Failed to make folder at path: {}", location));
        logging::error_log(&format!("folder_make: err {}", err));
    }
}

///
/// Finds a sidecar file
/// Doesn't check what type of sidecar it is
///
pub fn find_sidecar(location: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let sidecar_exts = ["txt", "json"];

    for ext in sidecar_exts {
        let test_path_location = location.display().to_string() + "." + ext;
        let test_path = Path::new(&test_path_location);
        if test_path.exists() {
            out.push(test_path.to_path_buf());
        }
    }

    out
}

///
/// Parses a sidecar file into a valid data for the db
///
pub fn parse_sidecar(file_location: &Path, sidecar_location: &Path, db: Arc<RwLock<Main>>) {
    let sha512hash = hash_file(
        &file_location.display().to_string(),
        &sharedtypes::HashesSupported::Sha512("Null".into()),
    );
    match sha512hash {
        Err(err) => {
            logging::error_log(&format!(
                "Cannot parse file: {} due to error: {:?}",
                file_location.display(),
                err
            ));
        }

        Ok((sha512hash, bytes)) => {
            let tag_list;
            if let Ok(filetype) = file_format::FileFormat::from_file(sidecar_location) {
                if filetype.extension() == "txt" {
                    tag_list = parse_sidecar_txt(sidecar_location);
                } else if filetype.extension() == "json" {
                    tag_list = parse_sidecar_json(sidecar_location);
                } else {
                    return;
                }
                let mut unwrappy = db.write().unwrap();
                unwrappy.enclave_determine_processing(
                    &mut sharedtypes::FileObject {
                        source_url: None,
                        hash: sharedtypes::HashesSupported::Sha512(sha512hash.clone()),
                        tag_list,
                        skip_if: vec![sharedtypes::SkipIf::FileHash(sha512hash.clone())],
                    },
                    bytes,
                    &sha512hash,
                    None,
                );
            }
        }
    }
}

pub fn parse_sidecar_txt(sidecar_location: &Path) -> Vec<sharedtypes::TagObject> {
    let mut out = Vec::new();

    if let Ok(file) = read_lines(sidecar_location) {
        for line in file.map(Result::ok) {
            match line {
                None => {}
                Some(line) => {
                    out.push(sharedtypes::TagObject {
                        namespace: sharedtypes::GenericNamespaceObj {
                            name: "SYSTEM_Sidecar_TXT".into(),
                            description: Some("Information from a sidecar file. TXT Import".into()),
                        },
                        tag: line,
                        tag_type: sharedtypes::TagType::Normal,
                        relates_to: None,
                    });
                }
            }
        }
    }

    out
}

pub fn parse_sidecar_json(sidecar_location: &Path) -> Vec<sharedtypes::TagObject> {
    let mut out = Vec::new();

    out
}

/// Stolen from: https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
/// The output is wrapped in a Result to allow matching on errors.
/// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
