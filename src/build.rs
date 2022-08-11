use std::process::Command;
use std::path::Path;
use std::fs;
use std::env;

///
/// Will compile all scrapers into their files
///
fn main() {
    println!("FROM BUILD>RS");



    let scrapersdir: String = "./scrapers".to_string();
    let path = Path::new(&scrapersdir);
    let mut paths: Vec<String> = Vec::new();

    if !path.exists() {fs::create_dir_all(&scrapersdir);}

    let dirs = fs::read_dir(&scrapersdir).unwrap();

    for entry in dirs {
        //println!("Name: {}", &entry.unwrap().path().display());
        let root = entry.as_ref().unwrap().path();

        env::set_current_dir(&root);

        paths.push(format!("{}", entry.as_ref().unwrap().path().display()));
        println!("cd {}", entry.as_ref().unwrap().path().display());
        let cmd = Command::new("cargo build --release").spawn();
        println!("{:?}", cmd.stdout );
println!("{:?}", cmd.stderr );
println!("{:?}", cmd.stdin );
        println!("AFTER");
    }
    println!("{:?}", paths);
    panic!();
}
