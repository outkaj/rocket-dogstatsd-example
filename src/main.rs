#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(custom_derive)]

// References to the application's "crates", or libraries, are gathered here
extern crate rocket;
extern crate dogstatsd;
extern crate rusqlite;

// Import statements go here
use dogstatsd::{Client, Options};
use std::sync::Mutex;
use rocket::{Rocket, State};
use rusqlite::{Connection, Error};
use std::time::{Instant};

type DbConn = Mutex<Connection>;

// Create the database and insert the single "Datadog" entry
fn init_database(conn: &Connection) {
    conn.execute("CREATE TABLE entries (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL
                  )", &[])
        .expect("create entries table");
    conn.execute("INSERT INTO entries (id, name) VALUES ($1, $2)",
            &[&0, &"Datadog"])
        .expect("insert single entry into entries table");
}

// Create a route for Rocket and specify what response is returned
#[get("/")]
fn hello(db_conn: State<DbConn>) -> Result<String, Error>  {
    // Binds to 127.0.0.1:8000 for transmitting and sends to
    // 127.0.0.1:8125, the default dogstatsd address
    let custom_options = Options::new("127.0.0.1:8000", "127.0.0.1:8125", "analytics");
    let custom_client = Client::new(custom_options);
    // Create a tag incrementing web page views
    custom_client.incr("web.page_views", vec!["tag:web.page_views".into()])
        .unwrap_or_else(|e| println!("Encountered error: {}", e));
    let start_time = Instant::now();
    let result = db_conn.lock()
        .expect("db connection lock")
        .query_row("SELECT name FROM entries WHERE id = 0",
                   &[], |row| { row.get(0) });
    let end_time = Instant::now();
    let duration = end_time.duration_since(start_time).as_secs();
    custom_client.histogram("database.query.time", &duration.to_string(), vec!["tag:database.query.time".into()])
      .unwrap_or_else(|e| println!("Encountered error: {}", e));
    result
}

// Set up the database and mount the Rocket application
fn rocket() -> Rocket {
    // Open a new in-memory SQLite database.
    let conn = Connection::open_in_memory().expect("in memory db");
    // Initialize the `entries` table in the in-memory database.
    init_database(&conn);
    // Have Rocket manage the database pool.
    rocket::ignite()
        .manage(Mutex::new(conn))
        .mount("/", routes![hello])
}

fn main() {
    // Start the application
    rocket().launch();
    }
