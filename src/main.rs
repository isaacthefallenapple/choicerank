use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use askama::Template;
use serde::Deserialize;
use sqlx::{prelude::*, PgPool};

use vote::{Choices, Vote};

mod vote;

#[derive(Debug)]
struct PoolModel {
    db: PgPool,
    model: Arc<Mutex<Model>>,
}

impl PoolModel {
    fn new(db: PgPool, model: Model) -> Self {
        Self {
            db,
            model: Arc::new(Mutex::new(model)),
        }
    }

    fn model(&self) -> MutexGuard<'_, Model> {
        self.model.lock().unwrap()
    }
}

impl Clone for PoolModel {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            model: self.model.clone(),
        }
    }
}

#[derive(Debug)]
enum Model {
    Landing,
    NewVote { id: i32, title: String },
    Vote { id: i32, vote: vote::Vote },
}

type Request = tide::Request<PoolModel>;

#[shuttle_runtime::main]
async fn tide(
    #[shuttle_static_folder::StaticFolder(folder = "front")] static_folder: PathBuf,
    #[shuttle_aws_rds::Postgres(
        local_uri = "postgres://timob:{secrets.PASSWORD}@localhost:5432/choicerank"
    )]
    pool: PgPool,
) -> shuttle_tide::ShuttleTide<PoolModel> {
    let model = PoolModel::new(pool.clone(), Model::Landing);
    let mut app = tide::with_state(model.clone());

    app.with(tide::log::LogMiddleware::new());

    // app.at("/").get(|_| async { Ok("Hello, world!") });
    app.at("/").serve_file(static_folder.join("index.html"))?;
    app.at("/vote").nest({
        let mut api = tide::with_state(model.clone());
        api.at("/").get(vote);
        api.at("/new").serve_file(static_folder.join("new.html"))?;
        api.at("/new").post(new_vote);
        api.at("/:code/").get(vote);
        api.at("/:code/").post(submit_ranking);
        api
    });

    Ok(app.into())
}

#[derive(Deserialize)]
struct Join {
    code: String,
}

async fn new_vote(mut req: Request) -> tide::Result {
    let vote: Vote = dbg!(req.body_form().await?);
    let db = &req.state().db;

    let rec = sqlx::query!(
        r"INSERT INTO vote(title, choices, max_choices) values($1, $2, $3) RETURNING id",
        vote.title(),
        vote.choices().as_bytes(),
        vote.max_choices() as i32,
    )
    .fetch_one(db)
    .await?;

    let id = rec.id;

    *req.state().model() = Model::NewVote {
        id,
        title: vote.title().to_string(),
    };

    Ok(tide::Redirect::new(&format!("/vote/{id}/")).into())
}

async fn submit_ranking(mut req: Request) -> tide::Result {
    let id: i32 = req.param("code")?.parse().unwrap();

    let value: HashMap<String, String> = req.body_form().await?;

    let db = &req.state().db;

    let mut transaction = db.begin().await?;

    sqlx::query!(
        r"INSERT INTO ranking VALUES ($1, $2, $3)",
        id,
        value["name"],
        value["ranking"].as_bytes()
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(tide::StatusCode::Created.into())
}

async fn vote(req: Request) -> tide::Result {
    if let Ok(Join { code }) = req.query() {
        return Ok(tide::Redirect::new(&format!("/vote/{code}/")).into());
    }
    let code: i32 = req.param("code")?.parse().unwrap();

    let rec = sqlx::query!(
        r"SELECT title, choices, max_choices FROM vote WHERE ID = $1",
        code
    )
    .fetch_one(&req.state().db)
    .await?;

    let vote = Vote::new(
        rec.title,
        String::from_utf8(rec.choices).expect("choices aren't utf-8"),
        rec.max_choices,
    );

    return Ok(vote.into());
}
