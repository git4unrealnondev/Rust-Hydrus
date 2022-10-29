 #[derive(Debug)]
pub enum jobs {
    add,
    remove,
}

enum subcommand {
    search,
    job,
}

struct cli_to_db {

    search_terms: Vec<String>,

    job: jobs,
    command: subcommand,

}
