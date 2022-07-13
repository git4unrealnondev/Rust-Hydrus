use log::{error, info, warn};
//use rusqlite::ToSql;
pub use rusqlite::{params, types::Null, Connection, Result, Transaction};
pub use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use std::panic;
use std::path::Path;
use crate::vec_of_strings;

/// Returns an open connection to use.
pub fn dbinit(dbpath: &String) -> Connection {
    //Engaging Transaction Handling
    let res = Connection::open(&dbpath).unwrap();
    return res;
}

/// Holder of database self variables
pub struct Main {
    _dbpath: String,
    _conn: Connection,
    _vers: isize,
}

/// Contains DB functions.
impl Main {
    /// Sets up new db instance.
    pub fn new(connection: Connection, path: String, vers: isize) -> Self {
        //let path = String::from("./main.db");

        Main {
            _dbpath: path,
            _conn: connection,
            _vers: vers,
        }
    }

    /// Vacuums database. cleans everything.
    pub fn vacuum(&mut self) {
        self.execute("VACUUM".to_string());
    }

    ///Sets up first database interaction.
    ///Makes tables and does first time setup.
    pub fn first_db(&mut self) {
        //Checking if file exists. If doesn't then no write perms.
        let dbexists = Path::new(&self.get_db_loc()).exists();

        if !dbexists {
            panic!("No database write perms or file not created");
        }

        // Making File Table
        let mut name = "File".to_string();
        let mut keys = vec_of_strings!["id", "hash", "filename", "size"];
        let mut vals = vec_of_strings!["INTEGER", "TEXT", "TEXT", "REAL"];
        self.table_create(&name, &keys, &vals);

        // Making Relationship Table
        name = "Relationship".to_string();
        keys = vec_of_strings!["fileid", "tagid"];
        vals = vec_of_strings!["INTEGER", "INTEGER"];
        self.table_create(&name, &keys, &vals);

        // Making Tags Table
        name = "Tags".to_string();
        keys = vec_of_strings!["id", "name", "parents", "namespace"];
        vals = vec_of_strings!["INTEGER", "TEXT", "INTEGER", "INTEGER"];
        self.table_create(&name, &keys, &vals);

        // Making Parents Table
        name = "Parents".to_string();
        keys = vec_of_strings!["id", "name", "children", "namespace"];
        vals = vec_of_strings!["INTEGER", "TEXT", "TEXT", "INTEGER"];
        self.table_create(&name, &keys, &vals);

        // Making Namespace Table
        name = "Namespace".to_string();
        keys = vec_of_strings!["id", "name", "description"];
        vals = vec_of_strings!["INTEGER", "TEXT", "TEXT"];
        self.table_create(&name, &keys, &vals);

        // Making Settings Table
        name = "Settings".to_string();
        keys = vec_of_strings!["name", "pretty", "num", "param"];
        vals = vec_of_strings!["TEXT", "TEXT", "INTEGER", "TEXT"];
        self.table_create(&name, &keys, &vals);
    }
    pub fn updatedb(&mut self) {

        let a = vec_of_strings!["a", "b"];

        self.add_setting(
            "VERSION".to_string(),
            "Version that the database is currently on.".to_string(),
            self._vers,
            "None".to_string(),
        );
        info!("Set VERSION to 1.");
        self.add_setting(
            "DEFAULTRATELIMIT".to_string(),
            "None".to_string(),
            5,
            "None".to_string(),
        );
        self.add_setting(
            "FilesLoc".to_string(),
            "None".to_string(),
            0,
            "./Files/".to_string(),
        );
        self.add_setting(
            "DEFAULTUSERAGENT".to_string(),
            "None".to_string(),
            0,
            "DIYHydrus/5.0 (Windows NT x.y; rv:10.0) Gecko/20100101 DIYHydrus/10.0".to_string(),
        );
        self.transaction_flush();
    }

    ///
    /// Creates a table
    /// name: The table name
    /// key: List of Collumn lables.
    /// dtype: List of Collumn types. NOTE Passed into SQLITE DIRECTLY THIS IS BAD :C
    ///
    pub fn table_create(&mut self, name: &String, key: &Vec<String>, dtype: &Vec<String>) {
        //Sanity checking...
        assert_eq!(key.len(), dtype.len(), "Warning table create was 2 Vecs weren't balanced. Lengths: {} {}", key.len(), dtype.len());

        //Not sure if theirs a better way to dynamically allocate a string based on two vec strings at run time.
        //Let me know if im doing something stupid.
        let mut concat = true;
        let mut c = 0;
        let mut stocat = "".to_string();
        while concat {
            let ke = &key[c];
            let dt = &dtype[c];
            stocat = [stocat, ke.to_string(), " ".to_string(), dt.to_string(), ", ".to_string()].concat();
            c += 1;
            if c >= key.len()-1 {
                concat=false;
            }
        }
        let ke = &key[key.len()-1];
        let dt = &dtype[dtype.len()-1];

        let endresult = ["CREATE TABLE IF NOT EXISTS ".to_string(), name.to_string(), " (".to_string(), stocat, ke.to_string(), " ".to_string(), dt.to_string(),")".to_string()].concat();

        info!("Creating table as: {}", endresult);
        stocat = endresult;

        self.execute(stocat);
    }

    ///
    /// Sets advanced settings for journaling.
    /// NOTE Experimental badness
    ///
    pub fn db_open(&mut self) {
        //self.execute("PRAGMA journal_mode = MEMORY".to_string());
        self.execute("PRAGMA synchronous = OFF".to_string());
        info!("Setting synchronous = OFF");
        //println!("db_open");
    }
    ///
    /// Adds a setting to the Settings Table.
    /// name: str   , Setting name
    /// pretty: str , Fancy Flavor text optional
    /// num: u64    , unsigned u64 largest int is 18446744073709551615 smallest is 0
    /// param: str  , Parameter to allow (value)
    ///
    pub fn add_setting(&mut self, name: String, pretty: String, num: isize, param: String) {
        let temp: isize = -9999;
        let _ex = self._conn.execute(
            "INSERT INTO Settings(name, pretty, num, param) VALUES (?1, ?2, ?3, ?4)",
            params![
                &name,
                //Hella jank workaround. can only pass 1 type into a function without doing workaround.
                //This makes it work should be fine for one offs.
                if &pretty == "None" {
                    &Null as &dyn ToSql
                } else {
                    &pretty
                },
                if &num == &temp {
                    &Null as &dyn ToSql
                } else {
                    &num
                },
                if &param == "None" {
                    &Null as &dyn ToSql
                } else {
                    &param
                }
            ],
        );

        match _ex {
            Err(_ex) => {
                println!(
                    "add_setting: Their was an error with inserting {} into db. {}",
                    &name, &_ex
                );
                error!(
                    "add_setting: Their was an error with inserting {} into db. {}",
                    &name, &_ex
                );
            }
            Ok(_ex) => (),
        }
    }

    /// Starts a transaction for bulk inserts.
    pub fn transaction_start(&mut self) {
        self.execute("BEGIN".to_string());
    }

    /// Flushes to disk.
    pub fn transaction_flush(&mut self) {
        self.execute("COMMIT".to_string());
        self.execute("BEGIN".to_string());
    }

    // Closes a transaction for bulk inserts.
    pub fn transaction_close(&mut self) {
        self.execute("COMMIT".to_string());
    }

    /// Pushes writes to db.
    pub fn write(&mut self) {
        //self._conn.execute("COMMIT", params![],).unwrap();
        let tx = self._conn.transaction().unwrap();
        tx.commit();
    }

    /// Returns db location as String refernce.
    pub fn get_db_loc(&self) -> String {
        return self._dbpath.to_string();
    }

    /// Raw Call to database. Try to only use internally to this file only.
    /// Doesn't support params nativly.
    /// Will not write changes to DB. Have to call write().
    /// Panics to help issues.
    pub fn execute(&mut self, inp: String) {
        let _out = self._conn.execute(&inp, params![]);

        match _out {
            Err(_out) => {
                println!("SQLITE STRING:: {}", inp);
                println!("BAD CALL {}", _out);
                error!("BAD CALL {}", _out);
                panic!("BAD CALL {}", _out);
            }
            Ok(_out) => (),
        }
    }

    /// Handles transactional pushes.
    pub fn transaction_execute(trans: Transaction, inp: String) {
        trans.execute(&inp, params![]);
    }
}
