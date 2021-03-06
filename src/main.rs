#[macro_use]
extern crate rocket;

use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use log::{info, error};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Capture {
    id: Option<u32>,
    content: String,
    created_at: Option<String>,
    processed_at: Option<String>,
}

#[rocket_sync_db_pools::database("main_db")]
struct DbConn(rocket_sync_db_pools::rusqlite::Connection);

#[get("/")]
async fn index(db: DbConn) -> Json<Vec<Capture>> {
    Json(load_captures(&db).await)
}

async fn load_captures(db: &DbConn) -> Vec<Capture> {
    db.run(|conn| {
      let mut stmt = conn
          .prepare(
              "SELECT id, content, created_at, processed_at FROM capture WHERE processed_at IS NULL ORDER BY created_at ASC",
          )
          .unwrap();
      stmt.query_map([], |row| Ok(Capture {
          id: row.get(0).unwrap(),
          content: row.get(1).unwrap(),
          created_at: row.get(2).unwrap(),
          processed_at: row.get(3).unwrap(),
      }))
      .unwrap()
      .map(|r| r.unwrap())
      .collect::<Vec<_>>()
    }).await
}

#[post("/", data="<capture>")]
async fn add_capture(db: DbConn, capture: Json<Capture>) {
    info!("Adding capture");
    db.run(move |conn| {
        conn.execute("INSERT INTO capture (content, created_at, processed_at) VALUES (?, CURRENT_TIMESTAMP, NULL)", &[&capture.content]).unwrap();
    }).await;
    info!("Added capture");
}

#[put("/processed/<id>")]
async fn mark_capture_processed(db: DbConn, id: u32) {
    info!("Marking capture {} as processed", id);
    db.run(move |conn| {
        let mut stmt = conn
            .prepare("UPDATE capture SET processed_at = CURRENT_TIMESTAMP WHERE id = ? AND processed_at IS NULL")
            .unwrap();

        match stmt.execute(&[&id]) {
            Ok(1) => info!("Marked capture {} as processed.", id),
            Ok(_) => error!("Could not mark capture {} as processed: did not match", id),
            Err(e) => error!("Could not mark capture {} as processed: {}", id, e),
        }
    })
    .await;
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(DbConn::fairing())
        .mount("/", routes![index, add_capture, mark_capture_processed])
}
