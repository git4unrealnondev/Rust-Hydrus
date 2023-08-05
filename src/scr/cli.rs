extern crate clap;
//use std::str::pattern::Searcher;

use crate::sharedtypes::{self, JobsAdd, JobsRemove};
use clap::{App, Arg, SubCommand};
use log::{error, info};
//use super::sharedtypes::;
use clap::parser::ValuesRef;
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
                        .exclusive(true)
                        .takes_value(true)
                        .help("Searches By File ID.")
                        .min_values(1)
                        .multiple_values(true),
                )
                .arg(
                    Arg::new("tid")
                        .long("tag_id")
                        .exclusive(true)
                        .takes_value(true)
                        .help("Searches By Tag Id.")
                        .min_values(1)
                        .multiple_values(true),
                )
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .exclusive(true)
                        .takes_value(true)
                        .help("Searches By Tag name needs namespace.")
                        .min_values(2)
                        .multiple_values(true),
                )
                .arg(
                    Arg::new("hash")
                        .long("hash")
                        .exclusive(true)
                        .takes_value(true)
                        .help("Searches By hash.")
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
                .subcommand(
                    SubCommand::with_name("csv")
                        .about("Imports a CSV File")
                        .arg(
                            Arg::new("csv")
                                .long("csv_file")
                                .takes_value(true)
                                .help("Location of csv import file.")
                                .min_values(1)
                                .multiple_values(false),
                        )
                        .arg(
                            Arg::new("Move")
                                .long("Move")
                                .takes_value(false)
                                .help("Moves files into db.")
                                .requires("csv"),
                        )
                        .arg(
                            Arg::new("Copy")
                                .long("Copy")
                                .takes_value(false)
                                .help("Copies files into db.")
                                .requires("csv"),
                        )
                        .arg(
                            Arg::new("Hardlink")
                                .long("Hardlink")
                                .takes_value(false)
                                .help("Hardlnks files into db.")
                                .requires("csv"),
                        ),
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
    //dbg!(&matches);

    //println!("{:?}", matches);
    let name = matches.value_of("site");

    let id = matches.value_of("id");

    let search = matches.subcommand_matches(&"search");

    let task = matches.subcommand_matches(&"task");

    if task != None {
        for taskenum in sharedtypes::Tasks::iter() {
            let tasktype = taskenum.to_string();
            let taskmatch = task.unwrap().subcommand_matches(&tasktype);
            if let Some(_) = taskmatch {
                match taskenum {
                    sharedtypes::Tasks::Csv(Test, csvdata) => {
                        dbg!(taskmatch);
                        let location: &String = taskmatch.unwrap().get_one(&tasktype).unwrap();

                        for csvdata in sharedtypes::CsvCopyMvHard::iter() {
                            if taskmatch.unwrap().contains_id(&csvdata.to_string()) {
                                dbg!(&csvdata);
                                match csvdata {
                                    sharedtypes::CsvCopyMvHard::Copy => {
                                        return sharedtypes::AllFields::Tasks(
                                            sharedtypes::Tasks::Csv(location.to_string(), csvdata),
                                        )
                                    }
                                    sharedtypes::CsvCopyMvHard::Move => {
                                        return sharedtypes::AllFields::Tasks(
                                            sharedtypes::Tasks::Csv(location.to_string(), csvdata),
                                        )
                                    }
                                    sharedtypes::CsvCopyMvHard::Hardlink => {
                                        return sharedtypes::AllFields::Tasks(
                                            sharedtypes::Tasks::Csv(location.to_string(), csvdata),
                                        )
                                    }
                                }
                                //return sharedtypes::AllFields::ETasks(csvdata())
                            }
                        }

                        dbg!(Test, csvdata);

                        let mv = taskmatch.unwrap().contains_id(&"move");
                        let cp = taskmatch.unwrap().contains_id(&"copy");
                        let hard = taskmatch.unwrap().contains_id(&"hardlink");

                        if mv {
                        } else if cp {
                        } else if hard {
                        }
                        dbg!(location, mv, cp);
                    }
                }
            }
        }
        panic!();
    }

    if search != None {
        for searchprog in sharedtypes::Search::iter() {
            let searchenumtype = searchprog.to_string();
            if search.unwrap().contains_id(&searchenumtype) {
                let retstring: Vec<String> = search
                    .unwrap()
                    .get_many::<String>(&searchenumtype)
                    .unwrap()
                    .map(|s| s.to_string())
                    .collect();

                match searchprog {
                    sharedtypes::Search::Fid(_) => {
                        return sharedtypes::AllFields::Search(sharedtypes::Search::Fid(retstring))
                    }
                    sharedtypes::Search::Tid(_) => {
                        return sharedtypes::AllFields::Search(sharedtypes::Search::Tid(retstring))
                    }
                    sharedtypes::Search::Tag(_) => {
                        return sharedtypes::AllFields::Search(sharedtypes::Search::Tag(retstring))
                    }
                    sharedtypes::Search::Hash(_) => {
                        return sharedtypes::AllFields::Search(sharedtypes::Search::Hash(
                            retstring,
                        ))
                    }
                }
            }
        }
    }

    if id != None {
        let valvec: Vec<String> = vec![id.unwrap().to_string()];
        //["Site", "Query", "Time", "Loop", "ReCommit"]
        let committype = sharedtypes::stringto_commit_type(&valvec[3]);
        return sharedtypes::AllFields::JobsAdd(JobsAdd {
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
                    return sharedtypes::AllFields::JobsAdd(JobsAdd {
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
                    return sharedtypes::AllFields::JobsRemove(JobsRemove {
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

            sharedtypes::AllFields::Nothing
            //(vec!["".to_string()], "".to_string(), false, false)
        }
    }

    // Extract the actual name
    //        let name = matches.value_of("name")
    //            .expect("This can't be None, we said it was required");

    //        println!("Hello, {}!", name);
}
