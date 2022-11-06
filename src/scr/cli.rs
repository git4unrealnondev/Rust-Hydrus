extern crate clap;
use crate::scr::sharedtypes::{self, jobs_add, jobs_remove};
use clap::{App, Arg, SubCommand};
use log::{error, info};
//use super::sharedtypes::;
use strum::IntoEnumIterator;
//mod sharedtypes;

pub fn main() -> sharedtypes::AllFields {
    let app = App::new("rust-hyrdrus")
        .version("1.0")
        .about("Das code sucks.")
        .author("git4unrealnondev")
        .arg(
            Arg::new("cfg")
                //.required_unless_present_all(&["dbg", "infile"])
                .takes_value(true)
                .long("config"),
        )
        .arg(Arg::new("dbg").long("debug"))
        .arg(Arg::new("infile").short('i').takes_value(true))
        .subcommand(
            SubCommand::with_name("job")
                .about("Manages their jobs in db.")
                .arg(
                    Arg::new("add")
                        .long("add")
                        .takes_value(true)
                        .help("Adds a job to the system")
                        .number_of_values(4)
                        .value_names(&["Site", "Query", "Time", "CommitType"])
                        .multiple_values(true),
                )
                .arg(
                    Arg::new("remove")
                        .long("remove")
                        .takes_value(true)
                        .help("Removes a job from the system")
                        .number_of_values(4)
                        .value_names(&["Site", "Query", "Time", "Loop"])
                        .multiple_values(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Searches the DB.")
                .arg(
                    Arg::new("fid")
                        .long("file_id")
                        .takes_value(true)
                        .help("Searches By File ID.")
                        .min_values(1)
                        .multiple_values(true),
                )
                .arg(
                    Arg::new("tid")
                        .long("tag_id")
                        .takes_value(true)
                        .help("Searches By Tag Id.")
                        .min_values(1)
                        .multiple_values(true),
                ),
        )
        .arg(
            Arg::new("id")
                //.required_unless_present_all(&["dbg", "infile"])
                .takes_value(true)
                .long("id"),
        )
        .subcommand(
            SubCommand::with_name("task")
                .about("Runs Specified tasks against DB.")
                .arg(
                    Arg::new("csv")
                        .long("csv_file")
                        .takes_value(true)
                        .help("Location of csv import file.")
                        .min_values(1)
                        .multiple_values(false),
                ),
        )
        .arg(
            Arg::new("site")
                .long("site")
                .takes_value(true)
                .help("Site to visit")
                .required(false),
        );

    // now add in the argument we want to parse
    //let app = app.arg();

    // extract the matches
    let matches = app.get_matches();

    //println!("{:?}", matches);
    let name = matches.value_of("site");

    let id = matches.value_of("id");

    //let search = matches.value_of("search");

    if id != None {
        let valvec: Vec<String> = vec![id.unwrap().to_string()];
        //["Site", "Query", "Time", "Loop", "ReCommit"]
        let committype = sharedtypes::stringto_commit_type(&valvec[3]);
        return sharedtypes::AllFields::EJobsAdd(jobs_add {
            site: valvec[0].to_owned(),
            query: valvec[1].to_owned(),
            time: valvec[2].to_owned(),
            committype: committype,
        });
        //return (valvec, "id".to_string(), true, false);
    }

    if name != None {
        println!("{}", name.unwrap());
    }

    match matches.subcommand() {
        /*Some(("task", subcmd)) => {

            let valvec: Vec<&String> = subcmd.get_many::<String>("csv").unwrap().collect();

            dbg!(valvec);

        }*/

        /*Some(("search", subcmd)) => {
            //let valvec: Vec<&String> = subcmd.get_many::<String>("add").unwrap().collect();
            let valret: Vec<String> = Vec::new();
            if subcmd.contains_id("fid") {
                dbg!("fid");
            }

            if subcmd.contains_id("tid") {
                dbg!("tid");
            }
            let radd = "".to_string();
            return (valret, radd, true, true);
        }*/
        Some(("job", subcmd)) => {
            if subcmd.contains_id("add") {
                let valvec: Vec<&String> = subcmd.get_many::<String>("add").unwrap().collect();
                //valret: Vec<String> = Vec::new();

                dbg!(&valvec, &valvec.len());

                let valret = [
                    valvec[0].to_owned(),
                    valvec[1].to_owned(),
                    valvec[2].to_owned(),
                    valvec[3].to_owned(),
                    //valvec[4].to_owned(),
                ]
                .to_vec();

                let lenjobs = 4;

                if valvec.len() != lenjobs {
                    println!("{:?}", valvec);
                    println!("{}", valvec[0]);

                    let msg: String = format!(
                        "WARNING: ONLY {} ARGUMENTS WERE SUPPLIED THEIR SHOULD OF BEEN: {} .",
                        valvec.len(),
                        lenjobs
                    );
                    error!("{}", msg);
                    panic!("{}", msg);
                } else {
                    //let radd = "add".to_string();
                    //if valvec[3] == "true" {
                    let committype = sharedtypes::stringto_commit_type(valvec[3]);
                    return sharedtypes::AllFields::EJobsAdd(jobs_add {
                        site: valvec[0].to_owned(),
                        query: valvec[1].to_owned(),
                        time: valvec[2].to_owned(),
                        committype: committype,
                    });
                    //return (valret, radd, true, true);
                    // }
                    //if valvec[3] == "false" {
                    //      return (valret, radd, false, true);
                    // }

                    //return (valret, radd, false, false);
                }
            }
            // Remove Job Handling
            if subcmd.contains_id("remove") {
                let valvec: Vec<&String> = subcmd.get_many::<String>("remove").unwrap().collect();
                let valret: Vec<String> = [
                    valvec[0].to_owned(),
                    valvec[1].to_owned(),
                    valvec[2].to_owned(),
                    valvec[3].to_owned(),
                ]
                .to_vec();
                let lenjobs = 4;

                if valvec.len() != lenjobs {
                    println!("{:?}", valvec);
                    println!("{}", valvec[0]);

                    let msg: String = format!(
                        "WARNING: ONLY {} ARGUMENTS WERE SUPPLIED THEIR SHOULD OF BEEN: {} .",
                        valvec.len(),
                        lenjobs
                    );
                    error!("{}", msg);
                    panic!("{}", msg);
                } else {
                    let rrmv = "remove".to_string();
                    let committype = sharedtypes::stringto_commit_type(valvec[3]);
                    return sharedtypes::AllFields::EJobsRemove(jobs_remove {
                        site: valvec[0].to_owned(),
                        query: valvec[1].to_owned(),
                        time: valvec[2].to_owned(),
                    });
                    //if valvec[3] == "true" {
                    //    return (valret, rrmv, true, true);
                    //}
                    //if valvec[3] == "false" {
                    //    return (valret, rrmv, false, true);
                    //}

                    //return (valret, rrmv, false, false);
                }
            }
            panic!("NO COMMANDS PASSED TO JOB.");
            //return ([&"".to_string(),&"".to_string()].to_vec(), false, false)
        }
        _ => {
            let msg = "No commands were passed into Rust-Hydrus.";
            println!("{}", msg);
            info!("{}", msg);

            sharedtypes::AllFields::ENothing
            //(vec!["".to_string()], "".to_string(), false, false)
        }
    }

    // Extract the actual name
    //        let name = matches.value_of("name")
    //            .expect("This can't be None, we said it was required");

    //        println!("Hello, {}!", name);
}
