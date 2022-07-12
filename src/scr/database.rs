use log::{error, info, warn};
use rusqlite::ToSql;
pub use rusqlite::{params, types::Null, Connection, Result, Transaction};
use std::panic;
use std::path::Path;
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

        self.execute(
            "CREATE TABLE if not exists File(
                                            id INTEGER,
                                            hash text,
                                            filename text,
                                            size real,
                                            ext text)"
                .to_string(),
        );
        self.execute(
            "CREATE TABLE Relationship(
                                            fileid INTEGER,
                                            tagid INTEGER)"
                .to_string(),
        );
        self.execute(
            "CREATE TABLE Tags(
                                            id INTEGER,
                                            name text,
                                            parents INTEGER,
                                            namespace INTEGER)"
                .to_string(),
        );
        self.execute(
            "CREATE TABLE Parents(
                                            id INTEGER,
                                            name text,
                                            children text,
                                            namespace INTEGER)"
                .to_string(),
        );
        self.execute(
            "CREATE TABLE Namespace(
                                            id INTEGER,
                                            name text,
                                            description text)"
                .to_string(),
        );
        self.execute(
            "CREATE TABLE Settings(
                                            name text,
                                            pretty text,
                                            num INTEGER,
                                            param text)"
                .to_string(),
        );
    }
    pub fn updatedb(&mut self) {
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

    /// Sets advanced settings for journaling.
    /// NOTE Experimental badness
    pub fn db_open(&mut self) {
        //self.execute("PRAGMA journal_mode = MEMORY".to_string());
        self.execute("PRAGMA synchronous = OFF".to_string());
        println!("db_open");
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
