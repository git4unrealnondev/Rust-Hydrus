use crate::database::database::{CacheType, Main};
use crate::error;
use crate::logging;
use crate::sharedtypes;
use crate::sharedtypes::DbParentsObj;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OptionalExtension;
use rusqlite::ToSql;
use rusqlite::Transaction;
use rusqlite::params;
use rusqlite::types::Null;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;

const DEFAULT_DURATION_BACKOFF: Duration = Duration::from_millis(100);

/// Waits until the calling function is OK then returns it
#[macro_export]
macro_rules! wait_until_sqlite_ok {
    ($expr:expr) => {{
        let mut cnt = 0;
        loop {
            match $expr {
                Ok(val) => break Ok(val),
                Err(err) => {
                    cnt += 1;
                    if cnt == 5 {
                        dbg!(&err);
                    }
                    // Retry only on "database is locked"
                    if let rusqlite::Error::SqliteFailure(_, Some(ref msg)) = err {
                        if msg.contains("database is locked") {
                            std::thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                    }
                    // Any other error — return it
                    break Err(err);
                }
            }
        }
    }};
}

/// Starts a transaction for bulk inserts.
pub fn transaction_start<'a>(
    conn: &'a mut PooledConnection<SqliteConnectionManager>,
) -> Transaction<'a> {
    //let mut con = self.pool.get().unwrap();
    conn.transaction().unwrap()
}

impl Main {
    /// Finds all tag ids where they dont hace a relationship
    pub fn get_empty_tagids(&self) -> HashSet<usize> {
        let sql = "SELECT t.id
FROM Tags t
WHERE t.count ==0 AND NOT EXISTS (
    SELECT 1 FROM Parents p
    WHERE p.tag_id = t.id
)
AND NOT EXISTS (
    SELECT 1 FROM Parents p
    WHERE p.relate_tag_id = t.id
)
AND NOT EXISTS (
    SELECT 1 FROM Parents p
    WHERE p.limit_to = t.id
);";
        let conn = self.get_database_connection();
        let mut stmt = conn.prepare(sql).unwrap();
        wait_until_sqlite_ok!(
            stmt.query_map(params![], |row| row.get::<_, usize>(0))
                .unwrap()
                .collect::<Result<HashSet<usize>, _>>()
        )
        .unwrap_or(HashSet::new())
    }

    /// Searches database for tag ids and count of the tag
    pub fn search_tags_sql(
        &self,
        search_string: &String,
        limit_to: &usize,
    ) -> Vec<(usize, usize)> {
        // Create the SQL query with a dynamic MATCH condition and limit
        let sql = r#"
        SELECT t.id, t.count
FROM Tags t
JOIN Tags_fts fts ON fts.rowid = t.id
WHERE Tags_fts MATCH ?
ORDER BY t.count DESC
LIMIT ?;    "#;

        let conn = self.get_database_connection();
        let mut stmt = conn.prepare(sql).unwrap();

        // Format the search string with the `*` suffix for prefix search (add it here, not in the query)
        let search_query = format!("{}*", search_string); // Append '*' before passing to the query

        // Build the parameter list for the query
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&search_query, &limit_to];

        // Convert the parameters vector to a slice and pass to query_map
        wait_until_sqlite_ok!(
            stmt.query_map(params.as_slice(), |row| {
                Ok((row.get::<_, usize>(0)?, row.get::<_, usize>(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<(usize, usize)>, _>>()
        )
        .unwrap_or(Vec::new())
    }
    ///
    /// Sets up relationship table and creates tagindex
    ///
    pub fn relationship_create_v2(&self, tn: &mut Transaction) {
        tn.execute(
            "CREATE TABLE IF NOT EXISTS Relationship (
    fileid INTEGER NOT NULL,
    tagid  INTEGER NOT NULL,

    PRIMARY KEY (fileid, tagid),

    FOREIGN KEY (fileid)
        REFERENCES File(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,

    FOREIGN KEY (tagid)
        REFERENCES Tags(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE
) WITHOUT ROWID;",
            [],
        )
        .unwrap();

        tn.execute("DROP INDEX IF EXISTS idx_tagid_fileid", [])
            .unwrap();

        tn.execute(
            "CREATE INDEX idx_tagid_fileid ON Relationship(tagid, fileid)",
            [],
        )
        .unwrap();
    }

    ///
    /// Creates the parents table and creates indexes
    ///
    pub fn parents_create_v2(&self, tn: &mut Transaction) {
        tn.execute(
            "CREATE TABLE IF NOT EXISTS Parents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tag_id INTEGER NOT NULL,
    relate_tag_id INTEGER NOT NULL,
    limit_to INTEGER,

    FOREIGN KEY (tag_id) REFERENCES Tags(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (relate_tag_id) REFERENCES Tags(id) ON DELETE CASCADE ON UPDATE CASCADE,
    FOREIGN KEY (limit_to) REFERENCES Tags(id) ON DELETE SET NULL ON UPDATE CASCADE,

    CHECK (tag_id != relate_tag_id)
);",
            [],
        )
        .unwrap();

        tn.execute(
            "CREATE INDEX IF NOT EXISTS idx_parents ON Parents (tag_id, relate_tag_id, limit_to)",
            [],
        )
        .unwrap();
        tn.execute(
            "CREATE INDEX IF NOT EXISTS idx_parents_rel ON Parents (relate_tag_id)",
            [],
        )
        .unwrap();
        tn.execute(
            "CREATE INDEX IF NOT EXISTS idx_parents_lim ON Parents (limit_to)",
            [],
        )
        .unwrap();
    }

    ///
    /// Creates tag Fast Text Search does a little parsing
    /// Adds triggers to keep it up to date
    ///
    pub fn tags_fts_create_v1(&self, tn: &mut Transaction) {
        tn.execute(
            "
        CREATE VIRTUAL TABLE IF NOT EXISTS Tags_fts USING fts5(
            name,
            namespace UNINDEXED,
            content='Tags',
            content_rowid='id',
            tokenize='unicode61'
        );
        ",
            [],
        )
        .unwrap();

        tn.execute(
            "
        INSERT INTO Tags_fts(rowid, name, namespace)
        SELECT id, replace(name, '_', ' '), namespace
        FROM Tags;
        ",
            [],
        )
        .unwrap();

        tn.execute(
            "
        CREATE TRIGGER IF NOT EXISTS Tags_ai AFTER INSERT ON Tags
        BEGIN
          INSERT INTO Tags_fts(rowid, name, namespace)
          VALUES (new.id, replace(new.name, '_', ' '), new.namespace);
        END;
        ",
            [],
        )
        .unwrap();

        tn.execute(
            "
        CREATE TRIGGER IF NOT EXISTS Tags_ad AFTER DELETE ON Tags
        BEGIN
          DELETE FROM Tags_fts WHERE rowid = old.id;
        END;
        ",
            [],
        )
        .unwrap();

        tn.execute(
            "
        CREATE TRIGGER IF NOT EXISTS Tags_au AFTER UPDATE ON Tags
        BEGIN
          UPDATE Tags_fts
          SET name = replace(new.name, '_', ' '), namespace = new.namespace
          WHERE rowid = old.id;
        END;
        ",
            [],
        )
        .unwrap();
            }
    /// Creates namespace properties table and its linker
    pub fn namespace_properties_create_v1(&self, tn: &mut Transaction) {
        tn.execute(
            "
CREATE TABLE IF NOT EXISTS NamespaceProperty (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT NOT NULL UNIQUE,  
  property_value TEXT NOT NULL,  
  description TEXT
);
",
            [],
        )
        .unwrap();
        tn.execute(
            "
CREATE TABLE IF NOT EXISTS NamespacePropertyLink (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  namespace_id INTEGER NOT NULL,
  property_id INTEGER NOT NULL,
    UNIQUE(namespace_id, property_id),
  FOREIGN KEY (namespace_id) REFERENCES Namespace(id) ON DELETE CASCADE,
  FOREIGN KEY (property_id) REFERENCES NamespaceProperty(id) ON DELETE CASCADE
);

",
            [],
        )
        .unwrap();
    }

    /// Creates the tags count table
    pub fn tag_count_create_v1(&self, tn: &mut Transaction) {
        tn.execute(
            "
ALTER TABLE Tags ADD COLUMN count INTEGER NOT NULL DEFAULT 0;
",
            [],
        )
        .unwrap();

        tn.execute(
            "

CREATE TRIGGER relationship_insert_count
AFTER INSERT ON Relationship
BEGIN
    UPDATE Tags
    SET count = count + 1
    WHERE id = NEW.tagid;
END;
",
            [],
        )
        .unwrap();
        tn.execute(
            "
CREATE TRIGGER relationship_delete_count
AFTER DELETE ON Relationship
BEGIN
    UPDATE Tags
    SET count = count - 1
    WHERE id = OLD.tagid;
END;",
            [],
        )
        .unwrap();

        tn.execute(
            "UPDATE Tags
SET count = sub.cnt
FROM (
    SELECT tagid, COUNT(*) AS cnt
    FROM Relationship
    GROUP BY tagid
) AS sub
WHERE Tags.id = sub.tagid;",
            [],
        )
        .unwrap();

tn.execute(
            "
        CREATE INDEX IF NOT EXISTS idx_tags_count ON Tags(count DESC);
        ",
            [],
        )
        .unwrap();

    }

    ///
    /// Gets all tagids where namespace_id has a linkage with a tag that has count greater then
    /// cnt
    ///
    pub fn relationship_get_tagid_where_namespace_count(
        &self,
        namespace_id: &usize,
        count: &usize,
        direction: &sharedtypes::GreqLeqOrEq,
    ) -> Vec<usize> {
        let dir = match direction {
            sharedtypes::GreqLeqOrEq::GreaterThan => '>',
            sharedtypes::GreqLeqOrEq::LessThan => '<',
            sharedtypes::GreqLeqOrEq::Equal => '=',
        };

        let conn = self.get_database_connection();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT t.id AS tagid
FROM Tags t
LEFT JOIN Relationship r ON r.tagid = t.id
WHERE t.namespace = ?
GROUP BY t.id
HAVING COUNT(r.fileid) {dir} ?;"
            ))
            .unwrap();
        wait_until_sqlite_ok!(
            stmt.query_map(params![namespace_id, count], |row| row.get::<_, usize>(0))
                .unwrap()
                .collect::<Result<Vec<usize>, _>>()
        )
        .unwrap_or(Vec::new())
    }

    ///
    /// Gets all fileids where namespace_id has a linkage with a tag that has count greater then
    /// cnt
    ///
    pub fn relationship_get_fileid_where_namespace_count(
        &self,
        namespace_id: &usize,
        count: &usize,
        direction: &sharedtypes::GreqLeqOrEq,
    ) -> Vec<usize> {
        let dir = match direction {
            sharedtypes::GreqLeqOrEq::GreaterThan => '>',
            sharedtypes::GreqLeqOrEq::LessThan => '<',
            sharedtypes::GreqLeqOrEq::Equal => '=',
        };
        dbg!(format!(
            "SELECT r.fileid FROM Relationship r LEFT JOIN Tags t ON r.tagid = t.id AND t.namespace = ? GROUP BY r.fileid HAVING COUNT(t.id) {dir} ?;"
        ));
        let conn = self.get_database_connection();
        let mut stmt = conn
            .prepare(
                &format!("SELECT r.fileid FROM Relationship r LEFT JOIN Tags t ON r.tagid = t.id AND t.namespace = ? GROUP BY r.fileid HAVING COUNT(t.id) {dir} ?;")
            )
            .unwrap();
        wait_until_sqlite_ok!(
            stmt.query_map(params![namespace_id, count], |row| row.get::<_, usize>(0))
                .unwrap()
                .collect::<Result<Vec<usize>, _>>()
        )
        .unwrap_or(Vec::new())
    }

    ///
    /// Searches for a list of fileids from a list of tagids
    /// Straight yoinked off chad
    ///
    pub fn relationship_get_fileid_search_sql(&self, tag_ids: &[usize]) -> Vec<usize> {
        if tag_ids.is_empty() {
            return vec![];
        }

        let tn = self.pool.get().unwrap();

        // 1️⃣ Deduplicate input tags
        let mut tag_ids: Vec<usize> = tag_ids.to_vec();
        tag_ids.sort_unstable();
        tag_ids.dedup();

        // 2️⃣ Sort tags by rarity (Tags.count ASC)
        let placeholders = std::iter::repeat_n("?", tag_ids.len())
            .collect::<Vec<_>>()
            .join(", ");

        let count_sql = format!(
            "SELECT id
         FROM Tags
         WHERE id IN ({})
         ORDER BY count ASC",
            placeholders
        );

        let mut stmt = tn.prepare(&count_sql).unwrap();

        let sorted_tag_ids: Vec<usize> = stmt
            .query_map(rusqlite::params_from_iter(&tag_ids), |row| {
                row.get::<_, usize>(0)
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // If any tag is missing → no possible matches
        if sorted_tag_ids.len() != tag_ids.len() {
            return vec![];
        }

        // 3️⃣ Main query (match ALL tags)
        let placeholders = std::iter::repeat_n("?", sorted_tag_ids.len())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT fileid
         FROM Relationship
         WHERE tagid IN ({})
         GROUP BY fileid
         HAVING COUNT(tagid) = ?
         LIMIT 500",
            placeholders
        );

        let mut stmt = tn.prepare(&sql).unwrap();

        let mut params: Vec<&dyn ToSql> = sorted_tag_ids.iter().map(|t| t as &dyn ToSql).collect();

        let required_count = sorted_tag_ids.len();
        params.push(&required_count);

        stmt.query_map(&params[..], |row| row.get::<_, usize>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|_| vec![])
    }
    ///
    /// Checks if a relationship exists
    ///
    pub fn relationship_exists(&self, file_id: &usize, tag_id: &usize) -> bool {
        let sql = "SELECT EXISTS(SELECT 1 FROM Relationship WHERE fileid=? AND tagid=? LIMIT 1)";

        let tn = match self.pool.get() {
            Ok(conn) => conn,
            Err(_) => return false,
        };

        let result: Result<i32, _> = tn.query_one(sql, params![file_id, tag_id], |row| {
            row.get::<usize, i32>(0) // specify column index and expected type
        });

        match result {
            Ok(exists) => exists == 1,
            Err(_) => false,
        }
    }

    ///
    /// Gets all jobs from the sql tables
    ///
    pub fn jobs_get_all_sql(&self) -> HashMap<usize, sharedtypes::DbJobsObj> {
        let mut out = HashMap::new();
        let tn = self.pool.get().unwrap();
        let max_jobs = self.jobs_return_count_sql();

        for job_id in 0..max_jobs {
            if let Some(job) = self.jobs_get_id_sql(&job_id) {
                out.insert(job_id, job.clone());
            }
        }

        out
    }

    ///
    /// Returns the total count of the jobs table
    ///
    pub fn jobs_return_count_sql(&self) -> usize {
        let tn = self.pool.get().unwrap();
        let mut max: Option<usize> =
            wait_until_sqlite_ok!(
                tn.query_row("SELECT MAX(id) FROM Jobs", params![], |row| row.get(0))
            )
            .unwrap_or(Some(0));
        let mut count =
            wait_until_sqlite_ok!(
                tn.query_row("SELECT COUNT(*) FROM Jobs", params![], |row| row.get(0))
            )
            .unwrap_or(Some(0));

        if max.is_none() {
            max = Some(0);
        }

        if count.is_none() {
            count = Some(0);
        }

        if let (Some(max), Some(count)) = (max, count) {
            if max > count { max + 1 } else { count + 1 }
        } else {
            0
        }
    }
    ///
    /// Returns the total count of the namespace table
    ///
    pub fn namespace_return_count_sql(&self) -> usize {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(
            tn.query_row("SELECT COUNT(*) FROM Namespace", params![], |row| {
                row.get(0)
            })
        )
        .unwrap_or(0)
    }

    ///
    /// Get file if it exists by id
    ///
    pub fn files_get_id_sql(&self, file_id: &usize) -> Option<sharedtypes::DbFileStorage> {
        let tn = self.pool.get().unwrap();
        let inp = "SELECT * FROM File where id = ?";
        wait_until_sqlite_ok!(tn.query_row(inp, params![file_id], |row| {
            let id = row.get(0).unwrap();
            let hash = row.get(1).unwrap();
            let ext_id = row.get(2).unwrap();
            let storage_id = row.get(3).unwrap_or(sharedtypes::DEFAULT_PRIORITY);
            Ok(Some(sharedtypes::DbFileStorage::Exist(
                sharedtypes::DbFileObj {
                    id,
                    hash,
                    ext_id,
                    storage_id,
                },
            )))
        }))
        .unwrap_or(None)
    }

    ///
    /// Returns all namespace keys
    ///
    pub fn namespace_keys_sql(&self) -> Vec<usize> {
        let tn = self.pool.get().unwrap();
        let mut out = Vec::new();
        let mut inp = tn.prepare("SELECT id FROM Namespace").unwrap();
        let quer = inp.query_map(params![], |row| row.get(0)).unwrap();

        for each in quer.flatten() {
            out.push(each);
        }

        out
    }

    ///
    /// Get file if it exists by id
    ///
    pub fn namespace_get_tagids_sql(&self, ns_id: &usize) -> HashSet<usize> {
        let tn = self.pool.get().unwrap();
        let mut out = HashSet::new();
        let mut inp = tn
            .prepare("SELECT id FROM Tags where namespace = ?")
            .unwrap();
        let quer = wait_until_sqlite_ok!(inp.query_map(params![ns_id], |row| {
            let id = row.get(0).unwrap();
            Ok(id)
        }))
        .unwrap();

        for each in quer.flatten() {
            out.insert(each);
        }

        out
    }

    ///
    /// Gets a job by id
    ///
    pub fn jobs_get_id_sql(&self, job_id: &usize) -> Option<sharedtypes::DbJobsObj> {
        let tn = self.pool.get().unwrap();
        let inp = "SELECT * FROM Jobs WHERE id = ? LIMIT 1";
        wait_until_sqlite_ok!(tn.query_row(inp, params![job_id], |row| {
            let id = row.get(0).unwrap();
            let time = row.get(1).unwrap();
            let reptime = row.get(2).unwrap();
            let priority = row.get(3).unwrap_or(sharedtypes::DEFAULT_PRIORITY);
            let cachetime = row.get(4).unwrap_or_default();
            let cachechecktype: String = row.get(5).unwrap();
            let manager: String = row.get(6).unwrap();
            let man = serde_json::from_str(&manager).unwrap();
            let site = row.get(7).unwrap();
            let param: String = row.get(8).unwrap();
            let system_data_string: String = row.get(9).unwrap();
            let user_data_string: String = row.get(10).unwrap();
            let system_data = serde_json::from_str(&system_data_string).unwrap();
            let user_data = serde_json::from_str(&user_data_string).unwrap();
            Ok(Some(sharedtypes::DbJobsObj {
                id,
                time,
                reptime,
                priority,
                cachetime,
                cachechecktype: serde_json::from_str(&cachechecktype).unwrap(),
                site,
                param: serde_json::from_str(&param).unwrap(),
                jobmanager: man,
                isrunning: false,
                user_data,
                system_data,
            }))
        }))
        .unwrap_or(None)
    }

    /// Adds a job to sql
    pub fn jobs_add_sql(&self, data: &sharedtypes::DbJobsObj) {
        self.transaction_start();
        let tn = self.write_conn.lock();
        let inp = "INSERT INTO Jobs VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";
        {
            wait_until_sqlite_ok!(tn.execute(
                inp,
                params![
                    data.id.unwrap().to_string(),
                    data.time.to_string(),
                    data.reptime.to_string(),
                    data.priority.to_string(),
                    serde_json::to_string(&data.cachetime).unwrap(),
                    serde_json::to_string(&data.cachechecktype).unwrap(),
                    serde_json::to_string(&data.jobmanager).unwrap(),
                    data.site,
                    serde_json::to_string(&data.param).unwrap(),
                    serde_json::to_string(&data.system_data).unwrap(),
                    serde_json::to_string(&data.user_data).unwrap(),
                ],
            ))
            .unwrap();
        }
    }

    /// Wrapper that handles inserting parents info into DB.
    pub fn parents_add_sql(&self, parent: &sharedtypes::DbParentsObj) -> usize {
        let tn = self.write_conn.lock();
        let inp = "INSERT INTO Parents(tag_id, relate_tag_id, limit_to) VALUES(?, ?, ?)";
        let limit_to = match parent.limit_to {
            None => &Null as &dyn ToSql,
            Some(out) => &out.to_string(),
        };
        {
            let _ = wait_until_sqlite_ok!(tn.execute(
                inp,
                params![
                    parent.tag_id.to_string(),
                    parent.relate_tag_id.to_string(),
                    limit_to
                ],
            ));
            return wait_until_sqlite_ok!(tn.query_one(
                "SELECT id FROM Parents WHERE tag_id = ? AND relate_tag_id = ? LIMIT 1",
                params![parent.tag_id.to_string(), parent.relate_tag_id.to_string(),],
                |one| one.get(0),
            ))
            .unwrap();
        }
    }

    ///
    /// Returns a list of parents where: relate_tag_id
    /// exists
    ///
    pub fn parents_relate_tag_get(&self, relate_tag: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        let tn = self.pool.get().unwrap();
        let mut out = HashSet::new();

        let mut stmt = tn
            .prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents WHERE relate_tag_id = ?")
            .unwrap();
        let temp = wait_until_sqlite_ok!(stmt.query_map(params![relate_tag], |row| {
            let tag_id: usize = row.get(0).unwrap();
            let relate_tag_id: usize = row.get(1).unwrap();
            let limit_to: Option<usize> = row.get(2).unwrap();

            Ok(sharedtypes::DbParentsObj {
                tag_id,
                relate_tag_id,
                limit_to,
            })
        }))
        .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of parents where: tag_id
    /// exists
    ///
    pub fn parents_tagid_tag_get(&self, tag_id: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        let tn = self.pool.get().unwrap();
        let mut out = HashSet::new();

        let mut stmt = tn
            .prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents WHERE tag_id = ?")
            .unwrap();
        let temp = wait_until_sqlite_ok!(stmt.query_map(params![tag_id], |row| {
            let tag_id: usize = row.get(0).unwrap();
            let relate_tag_id: usize = row.get(1).unwrap();
            let limit_to: Option<usize> = row.get(2).unwrap();

            Ok(sharedtypes::DbParentsObj {
                tag_id,
                relate_tag_id,
                limit_to,
            })
        }))
        .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of relate_tag_ids where: tag_id
    /// exists
    ///
    pub fn parents_tagid_get(&self, relate_tag: &usize) -> HashSet<usize> {
        let tn = self.pool.get().unwrap();
        let mut out = HashSet::new();

        let mut stmt = tn
            .prepare("SELECT tag_id FROM Parents WHERE relate_tag_id = ?")
            .unwrap();
        let temp = wait_until_sqlite_ok!(stmt.query_map(params![relate_tag], |row| {
            let tag_id: usize = row.get(0).unwrap();

            Ok(tag_id)
        }))
        .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of relate_tag_ids where: tag_id
    /// exists
    ///
    pub fn parents_relatetagid_get(&self, tag_id: &usize) -> HashSet<usize> {
        let tn = self.pool.get().unwrap();
        let mut out = HashSet::new();

        let mut stmt = tn
            .prepare("SELECT relate_tag_id FROM Parents WHERE tag_id = ?")
            .unwrap();
        let temp = wait_until_sqlite_ok!(stmt.query_map(params![tag_id], |row| {
            let relate_tag_id: usize = row.get(0).unwrap();

            Ok(relate_tag_id)
        }))
        .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    ///
    /// Returns a list of parents where: limit_to
    /// exists
    ///
    pub fn parents_limitto_tag_get(&self, limitto: &usize) -> HashSet<sharedtypes::DbParentsObj> {
        let tn = self.pool.get().unwrap();
        let mut out = HashSet::new();

        let mut stmt = tn
            .prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents WHERE limit_to = ?")
            .unwrap();
        let temp = wait_until_sqlite_ok!(stmt.query_map(params![limitto], |row| {
            let tag_id: usize = row.get(0).unwrap();
            let relate_tag_id: usize = row.get(1).unwrap();
            let limit_to: Option<usize> = row.get(2).unwrap();

            Ok(sharedtypes::DbParentsObj {
                tag_id,
                relate_tag_id,
                limit_to,
            })
        }))
        .unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    pub fn parents_delete_sql(&self, id: &usize) {
        self.parents_delete_tag_id_sql(id);
        self.parents_delete_relate_tag_id_sql(id);
        self.parents_delete_limit_to_sql(id);
    }

    ///
    /// Checks if a dead source exists
    ///
    pub fn does_dead_source_exist(&self, url: &String) -> bool {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id from dead_source_urls WHERE dead_url = ?",
            params![url],
            |row| Ok(row.get(0).unwrap_or(false)),
        ))
        .unwrap_or(false)
    }

    ///
    /// Does namespace contains tagid. A more optimizes sqlite version
    ///
    pub fn namespace_contains_id_sql(&self, tid: &usize, nsid: &usize) -> bool {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id FROM Tags WHERE id = ? AND namespace = ?",
            params![tid, nsid],
            |row| Ok(row.get(0).unwrap_or(false)),
        ))
        .unwrap_or(false)
    }

    ///
    /// Removes ALL of a tag_id from the parents collumn
    ///
    pub fn parents_delete_tag_id_sql(&self, tag_id: &usize) {
        let tn = self.write_conn.lock();
        let _ = wait_until_sqlite_ok!(
            tn.execute("DELETE FROM Parents WHERE tag_id = ?", params![tag_id])
        );
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    pub fn parents_delete_relate_tag_id_sql(&self, relate_tag_id: &usize) {
        let tn = self.write_conn.lock();
        let _ = wait_until_sqlite_ok!(tn.execute(
            "DELETE FROM Parents WHERE relate_tag_id = ?",
            params![relate_tag_id],
        ));
    }

    ///
    /// Removes ALL of a relate_tag_id from the parents collumn
    ///
    pub fn parents_delete_limit_to_sql(&self, limit_to: &usize) {
        let tn = self.write_conn.lock();
        let _ = wait_until_sqlite_ok!(
            tn.execute("DELETE FROM Parents WHERE limit_to = ?", params![limit_to])
        );
    }

    ///
    /// Gets a file storage location id
    ///
    pub fn storage_get_id(&self, location: &String) -> Option<usize> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id from FileStorageLocations where location = ?",
            params![location],
            |row| row.get(0),
        ))
        .optional()
        .unwrap_or_default()
    }

    ///
    /// Gets tag count
    /// Note needs to be offset by one because sqlite starts at 1 but the internal sqlite counter
    /// starts at zero but the stupid actual count starts at 1
    ///
    pub fn tags_max_return_sql(&self) -> usize {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row("SELECT MAX(id) FROM Tags", params![], |row| row.get(0)))
            .unwrap_or(0)
            + 1
    }

    ///
    /// Gets a tag by id
    ///
    pub fn tags_get_dbtagnns_sql(&self, tag_id: &usize) -> Option<sharedtypes::DbTagNNS> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT name, namespace from Tags where id = ?",
            params![tag_id],
            |row| match (row.get(0), row.get(1)) {
                (Ok(Some(name)), Ok(Some(namespace_id))) => Ok(Some(sharedtypes::DbTagNNS {
                    name,
                    namespace: namespace_id,
                })),
                _ => Ok(None),
            },
        ))
        .optional()
        .unwrap()?
    }

    ///
    /// Gets a list of tag ids
    ///
    pub fn tags_get_id_list_sql(&self) -> HashSet<usize> {
        let tn = self.pool.get().unwrap();
        let inp = "SELECT id FROM Tags";

        let mut stmt = tn.prepare(inp).unwrap();
        let temp =
            wait_until_sqlite_ok!(stmt.query_map([], |row| Ok(row.get(0).unwrap()))).unwrap();

        let mut out = HashSet::new();
        for item in temp {
            out.insert(item.unwrap());
        }
        out
    }

    ///
    /// Gets a list of tag ids
    ///
    pub fn file_get_list_id_sql(&self) -> HashSet<usize> {
        let tn = self.pool.get().unwrap();
        let inp = "SELECT id FROM File";

        let mut stmt = tn.prepare(inp).unwrap();
        let temp =
            wait_until_sqlite_ok!(stmt.query_map([], |row| Ok(row.get(0).unwrap()))).unwrap();

        let mut out = HashSet::new();
        for item in temp {
            out.insert(item.unwrap());
        }
        out
    }

    ///
    /// Gets a string from the ID of the storage location
    ///
    pub fn storage_get_string(&self, id: &usize) -> Option<String> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT location from FileStorageLocations where id = ?",
            params![id],
            |row| row.get(0),
        ))
        .optional()
        .unwrap_or_default()
    }

    ///
    /// Inserts into storage the location
    ///
    pub fn storage_put(&self, location: &String) -> usize {
        if let Some(out) = self.storage_get_id(location) {
            return out;
        }
        {
            let tn = self.write_conn.lock();
            let mut prep = tn
                .prepare("INSERT OR REPLACE INTO FileStorageLocations (location) VALUES (?)")
                .unwrap();

            wait_until_sqlite_ok!(prep.insert(params![location])).unwrap();
            wait_until_sqlite_ok!(tn.query_row(
                "SELECT id from FileStorageLocations where location = ?",
                params![location],
                |row| row.get(0),
            ))
            .unwrap()
        }
    }
    /// Adds tags into sql database
    pub(super) fn tag_add_sql(&self, tag_id: &usize, tag: &String, namespace: &usize) -> usize {
        let inp = "INSERT INTO Tags (id, name, namespace) VALUES(?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = EXCLUDED.name, namespace = EXCLUDED.namespace";
        {
            {
                let tn = self.write_conn.lock();
                let _ = wait_until_sqlite_ok!(tn.execute(inp, params![tag_id, tag, namespace]));
                wait_until_sqlite_ok!(tn.query_row(
                    "SELECT id FROM Tags WHERE name = ? AND namespace = ?",
                    params![tag, namespace],
                    |row| row.get(0),
                ))
                .unwrap()
            }
        }
    }

    /// Adds tags into sql database
    pub(super) fn tag_add_no_id_sql(&self, tag: &str, namespace: usize) -> usize {
        let sql = r#"
INSERT INTO Tags(name, namespace)
VALUES (?, ?)
ON CONFLICT(name, namespace) DO UPDATE SET name=excluded.name
RETURNING id;
"#;

        let conn = self.write_conn.lock();

        conn.query_row(sql, params![tag, namespace], |row| row.get(0))
            .unwrap()
    }

    /// Adds namespace to the SQL database
    pub(super) fn namespace_add_sql(
        &self,
        name: &String,
        description: &Option<String>,
        name_id: Option<usize>,
    ) {
        self.transaction_start();
        {
            let tn = self.write_conn.lock();
            let inp = "INSERT INTO Namespace (id, name, description) VALUES(?, ?, ?)";
            {
                let _ = wait_until_sqlite_ok!(tn.execute(inp, params![name_id, name, description]));
            }
        }
        self.transaction_flush();
    }

    /// Loads Parents in from DB tnection
    pub(super) fn load_parents(&self) {
        if matches!(self._cache, CacheType::Bare) {
            return;
        }
        logging::info_log("Database is Loading: Parents".to_string());
        let tn = self.get_database_connection();
        let temp = tn.prepare("SELECT tag_id, relate_tag_id, limit_to FROM Parents");
        if let Ok(mut con) = temp {
            let parents = wait_until_sqlite_ok!(con.query_map([], |row| {
                Ok(sharedtypes::DbParentsObj {
                    tag_id: row.get(0).unwrap(),
                    relate_tag_id: row.get(1).unwrap(),
                    limit_to: row.get(2).unwrap(),
                })
            }))
            .unwrap();
            for each in parents {
                if let Ok(res) = each {
                    self.parents_add_db(res);
                } else {
                    error!("Bad Parent cant load {:?}", each);
                }
            }
        }
    }

    ///
    /// Returns the parents
    ///
    pub fn parents_get_id_list_sql(&self, par: &sharedtypes::DbParentsObj) -> HashSet<usize> {
        let mut out = HashSet::new();
        let limit_to = match par.limit_to {
            None => &Null as &dyn ToSql,
            Some(temp) => &temp.clone() as &dyn ToSql,
        };

        let tn = self.get_database_connection();
        {
            let temp = match par.limit_to {
            None => {
                tn.prepare("SELECT id FROM Parents WHERE tag_id = ? AND relate_tag_id = ? ")
            }
            Some(_) => tn.prepare(
                "SELECT id FROM Parents WHERE tag_id = ? AND relate_tag_id = ? AND limit_to = ?",
            ),
        };

            if let Ok(mut con) = temp {
                match par.limit_to {
                    None => {
                        let parents = wait_until_sqlite_ok!(con.query_map(
                            [&par.tag_id as &dyn ToSql, &par.relate_tag_id as &dyn ToSql],
                            |row| {
                                let kep: usize = row.get(0).unwrap();

                                Ok(kep)
                            },
                        ))
                        .unwrap()
                        .flatten();

                        for each in parents {
                            let ear: usize = each;
                            out.insert(ear);
                        }
                    }

                    Some(lim) => {
                        let parents = wait_until_sqlite_ok!(con.query_map(
                            [
                                &par.tag_id as &dyn ToSql,
                                &par.relate_tag_id as &dyn ToSql,
                                &lim as &dyn ToSql,
                            ],
                            |row| {
                                let kep: usize = row.get(0).unwrap();

                                Ok(kep)
                            },
                        ))
                        .unwrap()
                        .flatten();
                        for each in parents {
                            let ear: usize = each;
                            out.insert(ear);
                        }
                    }
                }
            };
        }

        out
    }

    pub fn parents_dbobj_get_sql(&self, parent_id: &usize) -> Option<DbParentsObj> {
        let tn = self.get_database_connection();

        let result: Result<(usize, usize, Option<usize>), rusqlite::Error> = tn.query_row(
            "SELECT tag_id, relate_tag_id, limit_to FROM Parents WHERE id = ?",
            params![parent_id],
            |row| {
                Ok((
                    row.get::<_, usize>(0)?,
                    row.get::<_, usize>(1)?,
                    row.get::<_, Option<usize>>(2)?,
                ))
            },
        );

        match result {
            Ok((tag_id, relate_tag_id, limit_to)) => Some(DbParentsObj {
                tag_id,
                relate_tag_id,
                limit_to,
            }),
            Err(_) => None,
        }
    }

    pub fn file_tag_relationship(
        &self,
        fid: &usize,
        // tag, namespace
        tags: Vec<sharedtypes::TagObject>,
    ) {
        let tn = self.write_conn.lock();
        {
            // Prepare statements
            let mut ns_stmt = tn
                .prepare("INSERT INTO Namespace (name, description) VALUES (?1, ?2) ON CONFLICT(name) DO NOTHING")
                .unwrap();
            let mut get_ns_id_stmt = tn
                .prepare("SELECT id FROM Namespace WHERE name = ?1")
                .unwrap();
            let mut tag_stmt = tn
                .prepare(
                    "INSERT OR IGNORE INTO Tags (name, namespace) VALUES (?1, ?2)
         ",
                )
                .unwrap();
            let mut rel_stmt = tn
                .prepare("INSERT OR IGNORE INTO Relationship (fileid, tagid) VALUES (?1, ?2)")
                .unwrap();

            for tag in tags {
                // Insert namespace if missing
                ns_stmt
                    .execute(params![tag.namespace.name, tag.namespace.description])
                    .unwrap();

                // Get namespace id
                let namespace_id: i64 = get_ns_id_stmt
                    .query_row(params![tag.namespace.name], |row| row.get(0))
                    .unwrap();

                // Insert tag
                tag_stmt.execute(params![tag.tag, namespace_id]).unwrap();

                // Get tag id
                let tag_id: i64 = tn
                    .query_row(
                        "SELECT id FROM Tags WHERE name = ?1 AND namespace = ?2",
                        params![tag.tag, namespace_id],
                        |row| row.get(0),
                    )
                    .unwrap();

                // Insert file -> tag relationship
                rel_stmt.execute(params![fid, tag_id]).unwrap();
            }
        }
    }

    ///
    /// Convience function to search by extension strings
    ///
    pub fn extensions_get_fileid_extstr_sql(&self, extensions: &[String]) -> HashSet<usize> {
        let mut ext_id_vec = Vec::new();
        for ext in extensions.iter() {
            if let Some(ext_id) = self.extension_get_id(ext) {
                ext_id_vec.push(ext_id);
            }
        }
        self.extension_get_fileid_extid_sql(&ext_id_vec)
    }

    ///
    /// Gets all fileids where a list of extension ids exist
    ///
    pub fn extension_get_fileid_extid_sql(&self, extensions: &[usize]) -> HashSet<usize> {
        let conn = self.pool.get().unwrap();
        if extensions.is_empty() {
            return HashSet::new();
        }

        let placeholders = std::iter::repeat("?")
            .take(extensions.len())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT id
         FROM File
         WHERE extension IN ({})",
            placeholders
        );

        let mut stmt = conn.prepare(&sql).unwrap();

        wait_until_sqlite_ok!(
            stmt.query_map([], |row| row.get::<_, usize>(0))
                .unwrap()
                .collect::<Result<HashSet<usize>, _>>()
        )
        .unwrap_or(HashSet::new())
    }

    ///
    /// Adds a extension and an id OPTIONAL into the db
    ///
    pub fn extension_put_id_ext_sql(&self, id: Option<usize>, ext: &str) -> usize {
        let tn = self.write_conn.lock();
        {
            let _ = wait_until_sqlite_ok!(tn.execute(
                "insert or ignore into FileExtensions(id, extension) VALUES (?,?)",
                params![id, ext],
            ));
        }

        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id FROM FileExtensions WHERE extension = ?",
            params![ext],
            |row| row.get(0),
        ))
        .unwrap_or(None)
        .unwrap()
    }
    ///
    /// Returns id if a hash exists
    ///
    pub fn file_get_id_sql(&self, hash: &str) -> Option<usize> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id FROM File WHERE hash = ? LIMIT 1",
            params![hash],
            |row| row.get(0),
        ))
        .unwrap_or_default()
    }

    ///
    /// Returns if an extension exists gey by ext string
    ///
    pub fn extension_get_id_sql(&self, ext: &str) -> Option<usize> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id FROM FileExtensions WHERE extension = ?",
            params![ext],
            |row| row.get(0),
        ))
        .unwrap_or(None)
    }
    ///
    /// Returns if an extension exists get by id
    ///
    pub fn extension_get_string_sql(&self, id: &usize) -> Option<String> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "select extension from FileExtensions where id = ?",
            params![id],
            |row| row.get(0),
        ))
        .unwrap_or(None)
    }

    /// Adds file via SQL
    pub(super) fn file_add_sql(&self, file: &sharedtypes::DbFileStorage) -> usize {
        let out_file_id;
        let file_id;
        let hash;
        let extension;
        let storage_id;
        match file {
            sharedtypes::DbFileStorage::Exist(file) => {
                file_id = Some(file.id);
                hash = file.hash.clone();
                extension = file.ext_id;
                storage_id = file.storage_id;
            }
            sharedtypes::DbFileStorage::NoIdExist(file) => {
                file_id = None;
                hash = file.hash.clone();
                extension = file.ext_id;
                storage_id = file.storage_id;
            }
            sharedtypes::DbFileStorage::NoExist(_fid) => {
                todo!()
            }
            sharedtypes::DbFileStorage::NoExistUnknown => {
                todo!()
            }
        }

        // Catches issue where a non bare DB would nuke itself
        if matches!(self._cache, CacheType::Bare)
            && let Some(id) = self.file_get_hash(&hash)
        {
            return id;
        }

        let inp = "INSERT INTO File VALUES(?, ?, ?, ?)";
        {
            let tn = self.write_conn.lock();
            let _ = wait_until_sqlite_ok!(
                tn.execute(inp, params![file_id, hash, extension, storage_id])
            );
        }
        if let Some(id) = file_id {
            out_file_id = id;
        } else {
            let tn = self.write_conn.lock();

            out_file_id = wait_until_sqlite_ok!(tn.query_row(
                "SELECT id FROM File WHERE hash = ? LIMIT 1",
                params![hash],
                |row| row.get(0),
            ))
            .unwrap();
        }

        out_file_id
    }

    /// Loads Relationships in from DB tnection
    pub(super) fn load_relationships(&self) {
        //if self._cache == CacheType::Bare {
        if matches!(self._cache, CacheType::Bare) {
            return;
        }
        let tn = self.pool.get().unwrap();
        logging::info_log("Database is Loading: Relationships".to_string());
        let temp = tn.prepare("SELECT fileid, tagid FROM Relationship");
        if let Ok(mut con) = temp {
            let relationship = wait_until_sqlite_ok!(con.query_map([], |row| {
                Ok(sharedtypes::DbRelationshipObj {
                    fileid: row.get(0).unwrap(),
                    tagid: row.get(1).unwrap(),
                })
            }))
            .unwrap();
            for each in relationship {
                match each {
                    Ok(res) => {
                        self.relationship_add_db(res.fileid, res.tagid);
                    }
                    Err(err) => {
                        error!("Bad relationship cant load");
                        err.to_string().contains("database disk image is malformed");
                        error!("DATABASE IMAGE IS MALFORMED PANICING rel {:?}", &err);
                        panic!("DATABASE IMAGE IS MALFORMED PANICING rel {:?}", &err);
                    }
                }
            }
        }
    }
    /// Adds relationship to SQL db.
    pub fn relationship_add_sql(&self, file: &usize, tag: &usize) {
        if self.relationship_exists(file, tag) {
            return;
        }

        let tn = self.write_conn.lock();
        let inp = "INSERT OR IGNORE INTO Relationship VALUES(?, ?)";
        let _out = tn.execute(inp, params![file, tag]);
    }
    /// Updates job by id
    pub fn jobs_update_by_id(&self, data: &sharedtypes::DbJobsObj) {
        self.transaction_start();
        let tn = self.write_conn.lock();
        let inp = "UPDATE Jobs SET id=?, time=?, reptime=?, Manager=?, priority=?,cachetime=?,cachechecktype=?, site=?, param=?, SystemData=?, UserData=? WHERE id = ?";
        let _ = tn.execute(
            inp,
            params![
                data.id.unwrap().to_string(),
                data.time.to_string(),
                data.reptime.to_string(),
                serde_json::to_string(&data.jobmanager).unwrap(),
                data.priority.to_string(),
                serde_json::to_string(&data.cachetime).unwrap(),
                serde_json::to_string(&data.cachechecktype).unwrap(),
                data.site,
                serde_json::to_string(&data.param).unwrap(),
                serde_json::to_string(&data.system_data).unwrap(),
                serde_json::to_string(&data.user_data).unwrap(),
                data.id.unwrap().to_string()
            ],
        );
    }

    ///
    /// Gets a list of fileid associated with a tagid
    ///
    pub fn relationship_get_fileid_sql(&self, tag_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();

        let tn = self.pool.get().unwrap();
        let mut stmt = tn
            .prepare("SELECT fileid from Relationship where tagid = ?")
            .unwrap();
        let temp =
            wait_until_sqlite_ok!(stmt.query_map(params![tag_id], |row| row.get(0))).unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }
    ///
    /// Gets a list of tagid associated with a fileid
    ///
    pub fn relationship_get_tagid_sql(&self, file_id: &usize) -> HashSet<usize> {
        let mut out = HashSet::new();

        let tn = self.pool.get().unwrap();
        let mut stmt = tn
            .prepare("SELECT tagid from Relationship where fileid = ?")
            .unwrap();
        let temp =
            wait_until_sqlite_ok!(stmt.query_map(params![file_id], |row| row.get(0))).unwrap();
        for item in temp.flatten() {
            out.insert(item);
        }
        out
    }

    /// Querys the db use this for select statements. NOTE USE THIS ONY FOR RESULTS
    /// THAT RETURN STRINGS
    /*pub fn quer_stra(&self, inp: String) -> Result<Vec<String>> {
        let binding = self.tn.lock();
        let mut toexec = binding.prepare(&inp).unwrap();
        let rows = wait_until_sqlite_ok!(toexec.query_map([], |row| row.get(0))).unwrap();
        let mut out = Vec::new();
        for each in rows {
            out.push(each.unwrap());
        }
        Ok(out)
    }*/
    /// Querys the db use this for select statements. NOTE USE THIS ONY FOR RESULTS
    /// THAT RETURN INTS
    pub fn quer_int(&self, inp: String) -> Vec<isize> {
        let tn = self.pool.get().unwrap();
        let mut toexec = tn.prepare(&inp).unwrap();
        let rows = wait_until_sqlite_ok!(toexec.query_map([], |row| row.get(0))).unwrap();
        let mut out: Vec<isize> = Vec::new();
        for each in rows {
            match each {
                Ok(temp) => {
                    out.push(temp);
                }
                Err(errer) => {
                    error!("Could not load {} Due to error: {:?}", &inp, errer);
                }
            }
        }
        out
    }

    ///
    /// Adds a setting into the db
    /// NOTE do not remove the transaction start here. Is needed for the db upgrade to restartup
    /// commits
    ///
    pub fn setting_add_sql(
        &self,
        name: String,
        pretty: &Option<String>,
        num: Option<usize>,
        param: &Option<String>,
    ) {
        self.transaction_start();
        {
            let tn = self.write_conn.lock();
            let _ex =
            wait_until_sqlite_ok!( tn                .execute(
                    "INSERT INTO Settings(name, pretty, num, param) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(name) DO UPDATE SET pretty=?2, num=?3, param=?4 ;",
                    params![
                        &name,
                        // Hella jank workaround. can only pass 1 type into a function without doing
                        // workaround. This makes it work should be fine for one offs.
                        if pretty.is_none() {
                            &Null as &dyn ToSql
                        } else {
                            &pretty
                        },
                        if num.is_none() {
                            &Null as &dyn ToSql
                        } else {
                            &num
                        },
                        if param.is_none() {
                            &Null as &dyn ToSql
                        } else {
                            &param
                        }
                    ],
                ));
            match _ex {
                Err(_ex) => {
                    println!(
                        "setting_add: Their was an error with inserting {} into db. {}",
                        &name, &_ex
                    );
                    error!(
                        "setting_add: Their was an error with inserting {} into db. {}",
                        &name, &_ex
                    );
                }
                Ok(_ex) => {}
            }
        }
        self.transaction_flush();
    }

    /// Loads settings into db
    pub(super) fn load_settings(&self) {
        logging::info_log("Database is Loading: Settings".to_string());
        {
            let tn = self.pool.get().unwrap();
            let temp = tn.prepare("SELECT * FROM Settings");
            match temp {
                Ok(mut con) => {
                    let settings = wait_until_sqlite_ok!(con.query_map([], |row| {
                        Ok(sharedtypes::DbSettingObj {
                            name: row.get(0)?,
                            pretty: row.get(1)?,
                            num: row.get(2)?,
                            param: row.get(3)?,
                        })
                    }))
                    .unwrap();
                    for each in settings {
                        if let Ok(res) = each {
                            self.setting_add_db(res.name, res.pretty, res.num, res.param);
                        } else {
                            error!("Bad Setting cant load {:?}", each);
                        }
                    }
                }
                Err(_) => return,
            };
        }
        self.transaction_flush();
    }

    pub(super) fn add_dead_url_sql(&self, url: &String) {
        self.transaction_start();
        let tn = self.write_conn.lock();
        let _ = wait_until_sqlite_ok!(tn.execute(
            "INSERT INTO dead_source_urls(dead_url) VALUES (?)",
            params![url],
        ))
        .unwrap();
    }
    ///
    /// Returns id if a tag exists
    ///
    pub fn tags_get_id_sql(&self, db_tag_nns: &sharedtypes::DbTagNNS) -> Option<usize> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id FROM Tags WHERE name = ? AND namespace = ?",
            params![db_tag_nns.name, db_tag_nns.namespace],
            |row| row.get(0),
        ))
        .unwrap_or(None)
    }

    ///
    /// Migrates a relationship's tag id
    ///
    pub fn migrate_relationship_tag_sql(&self, old_tag_id: &usize, new_tag_id: &usize) {
        let tn = self.write_conn.lock();
        wait_until_sqlite_ok!(tn.execute(
            "UPDATE OR IGNORE Relationship SET tagid = ? WHERE tagid = ?",
            params![new_tag_id, old_tag_id],
        ))
        .unwrap();
    }
    ///
    /// Migrates a relationship's tag id
    ///
    pub fn migrate_relationship_file_tag_sql(
        &self,
        file_id: &usize,
        old_tag_id: &usize,
        new_tag_id: &usize,
    ) {
        let tn = self.write_conn.lock();
        wait_until_sqlite_ok!(tn.execute(
            "UPDATE OR REPLACE Relationship SET tagid = ? WHERE tagid = ? AND fileid=?",
            params![new_tag_id, old_tag_id, file_id],
        ))
        .unwrap();
    }

    ///
    /// Returns id if a namespace exists
    ///
    pub fn namespace_get_id_sql(&self, namespace: &String) -> Option<usize> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT id FROM Namespace WHERE name = ?",
            params![namespace],
            |row| row.get(0),
        ))
        .unwrap_or(None)
    }
    ///
    /// Returns dbnamespace if a namespace id exists
    ///
    pub fn namespace_get_namespaceobj_sql(
        &self,
        ns_id: &usize,
    ) -> Option<sharedtypes::DbNamespaceObj> {
        let tn = self.pool.get().unwrap();
        wait_until_sqlite_ok!(tn.query_row(
            "SELECT * FROM Namespace WHERE id = ?",
            params![ns_id],
            |row| {
                if let (Ok(id), Ok(name)) = (row.get(0), row.get(1)) {
                    Ok(Some(sharedtypes::DbNamespaceObj {
                        id,
                        name,
                        description: row.get(2).unwrap(),
                    }))
                } else {
                    Ok(None)
                }
            },
        ))
        .unwrap()
    }

    ///
    /// Loads the DB into memory
    ///
    pub(super) fn load_dead_urls(&self) {
        logging::info_log("Database is Loading: dead_source_urls".to_string());

        let tn = self.pool.get().unwrap();
        let temp = tn.prepare("SELECT * FROM dead_source_urls");

        if let Ok(mut con) = temp {
            let tag = wait_until_sqlite_ok!(con.query_map([], |row| {
                let url: String = row.get(0).unwrap();
                Ok(url)
            }));
            match tag {
                Ok(tags) => {
                    for each in tags {
                        if let Ok(res) = each {
                            // if let Some(id) = self._inmemdb.tags_get_data(&res.id) {
                            // logging::info_log(&format!( "Already have tag {:?} adding {} {} {}", id,
                            // res.name, res.namespace, res.id )); continue;
                            // delete_tags.insert((res.name.clone(), res.namespace.clone())); }
                            self.add_dead_url(&res);
                        } else {
                            error!("Bad dead_source_url cant load {:?}", each);
                        }
                    }
                }
                Err(errer) => {
                    error!(
                        "WARNING COULD NOT LOAD dead_source_url: {:?} DUE TO ERROR",
                        errer
                    );
                }
            }
        }
    }

    /// Loads tags into db
    pub(super) fn load_tags(&self) {
        if matches!(self._cache, CacheType::Bare) {
            return;
        }
        logging::info_log("Database is Loading: Tags".to_string());

        // let mut delete_tags = HashSet::new();
        {
            let tn = self.pool.get().unwrap();
            let temp = tn.prepare("SELECT (id, name, namespace) FROM Tags");
            if let Ok(mut con) = temp {
                let tag = wait_until_sqlite_ok!(con.query_map([], |row| {
                    let name: String = match row.get(1) {
                        Ok(out) => out,
                        Err(err) => {
                            let temp: u64 = row.get(1).unwrap();
                            panic!();
                        }
                    };

                    Ok(sharedtypes::DbTagObjCompatability {
                        id: row.get(0).unwrap(),
                        name,
                        namespace: row.get(2).unwrap(),
                    })
                }));
                match tag {
                    Ok(tags) => {
                        for each in tags {
                            if let Ok(res) = each {
                                // if let Some(id) = self._inmemdb.tags_get_data(&res.id) {
                                // logging::info_log(&format!( "Already have tag {:?} adding {} {} {}", id,
                                // res.name, res.namespace, res.id )); continue;
                                // delete_tags.insert((res.name.clone(), res.namespace.clone())); }
                                self.tag_add(&res.name, res.namespace, Some(res.id));
                            } else {
                                error!("Bad Tag cant load {:?}", each);
                            }
                        }
                    }
                    Err(errer) => {
                        error!("WARNING COULD NOT LOAD TAG: {:?} DUE TO ERROR", errer);
                    }
                }
            }
        }
    }

    /// Sets advanced settings for journaling. NOTE Experimental badness
    pub fn db_open(&self) {
        self.transaction_start();
        let tn = self.write_conn.lock();
        let _ = wait_until_sqlite_ok!(tn.execute("PRAGMA secure_delete = 0", params![]));
        let _ = wait_until_sqlite_ok!(tn.execute("PRAGMA busy_timeout = 5000", params![]));
        let _ = wait_until_sqlite_ok!(tn.execute("PRAGMA journal_mode = WAL", params![]));
        let _ = wait_until_sqlite_ok!(tn.execute("PRAGMA synchronous = NORMAL", params![]));
        let _ = wait_until_sqlite_ok!(tn.execute("PRAGMA page_size = 8192", params![]));
        let _ = wait_until_sqlite_ok!(tn.execute("PRAGMA cache_size = -5000000", params![]));
        //let _ = wait_until_sqlite_ok!(tn.execute("PRAGMA wal_autocheckpoint = 10000", params![]));
    }

    /// Removes a job from sql table by id
    pub fn del_from_jobs_table_sql_better(&self, id: &usize) {
        self.transaction_start();
        {
            let tn = self.write_conn.lock();
            //let inp = "DELETE FROM Jobs WHERE id = ? LIMIT 1";
            let inp = "DELETE FROM Jobs WHERE id = ?";
            let _ = wait_until_sqlite_ok!(tn.execute(inp, params![id])).unwrap();
        }
    }

    /// Removes a tag from sql table by name and namespace
    pub fn del_from_tags_by_name_and_namespace(&self, name: &String, namespace: &String) {
        self.transaction_start();
        let tn = self.write_conn.lock();
        let inp = "DELETE FROM Tags WHERE name = ? AND namespace = ?";
        wait_until_sqlite_ok!(tn.execute(inp, params![name, namespace])).unwrap();
    }

    /// Sqlite wrapper for deleteing a relationship from table.
    pub fn delete_relationship_sql(&self, file_id: &usize, tag_id: &usize) {
        let tn = self.write_conn.lock();
        logging::log(format!(
            "Removing Relationship where fileid = {} and tagid = {}",
            file_id, tag_id
        ));

        let inp = "DELETE FROM Relationship WHERE fileid = ? AND tagid = ?";
        {
            wait_until_sqlite_ok!(
                tn.execute(inp, params![file_id.to_string(), tag_id.to_string()])
            )
            .unwrap();
        }
    }

    /// Sqlite wrapper for deleteing a parent from table.
    pub fn delete_parent_sql(&self, tag_id: &usize, relate_tag_id: &usize) {
        let tn = self.write_conn.lock();
        let inp = "DELETE FROM Parents WHERE tag_id = ? AND relate_tag_id = ?";
        {
            let _ = wait_until_sqlite_ok!(
                tn.execute(inp, params![tag_id.to_string(), relate_tag_id.to_string()])
            );
        }
    }

    /// Sqlite wrapper for deleteing a tag from table.
    pub fn delete_tag_sql(&self, tag_id: &usize) {
        logging::log(format!("Removing tag with id: {}", tag_id));
        let tn = self.write_conn.lock();
        let inp = "DELETE FROM Tags WHERE id = ?";
        {
            let _ = wait_until_sqlite_ok!(tn.execute(inp, params![tag_id.to_string()]));
        }
    }

    /// Sqlite wrapper for deleteing a tag from table.
    pub fn delete_namespace_sql(&self, namespace_id: &usize) {
        let tn = self.write_conn.lock();
        logging::info_log(format!(
            "Deleting namespace with id : {} from db",
            namespace_id
        ));
        let inp = "DELETE FROM Namespace WHERE id = ?";
        {
            let _ = wait_until_sqlite_ok!(tn.execute(inp, params![namespace_id.to_string()]));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::VERS;

    use super::*;

    fn setup_default_db() -> Main {
        let mut db = Main::new(None, VERS);
        let parents = [
            sharedtypes::DbParentsObj {
                tag_id: 1,
                relate_tag_id: 2,
                limit_to: Some(3),
            },
            sharedtypes::DbParentsObj {
                tag_id: 2,
                relate_tag_id: 3,
                limit_to: Some(4),
            },
            sharedtypes::DbParentsObj {
                tag_id: 3,
                relate_tag_id: 4,
                limit_to: Some(5),
            },
        ];
        for parent in parents {
            db.parents_add(parent);
        }
        db
    }

    #[test]
    fn sql_parents_add() {}
    #[test]
    fn sql_parents_del_tag_id() {}
    #[test]
    fn sql_parents_del_relate_tag_id() {}
    #[test]
    fn sql_parents_del_limit_to() {}

    #[test]
    fn tag_retrieve() {
        // let mut db = Main::new(None, VERS);
        //db.tag_add(&"te".to_string(), 0, true, None);
        //assert!(db.tag_get_name("te".to_string(), 0).is_some());
    }
}
