extern crate clap;

use clap::{App, Arg};

pub fn main() {let app = App::new("hello-clap")
        .version("1.0")
        .about("Das code sucks.")
        .author("git4unrealnondev");

    // Define the name command line option
    let name_option = Arg::with_name("name")
        .long("name") // allow --name
        .takes_value(true)
        .help("Who to say hello to")
        .required(false);

    // now add in the argument we want to parse
    let app = app.arg(name_option);

    // extract the matches
    let matches = app.get_matches();

    // Extract the actual name
//        let name = matches.value_of("name")
//            .expect("This can't be None, we said it was required");

//        println!("Hello, {}!", name);
}
