use std::path::PathBuf;

use serde::Deserialize;
use sqlx::PgPool;

use vote::Vote;

mod vote;

#[derive(Debug, Clone)]
struct Model {
    db: PgPool,
}

type Request = tide::Request<Model>;

#[shuttle_runtime::main]
async fn tide(
    #[shuttle_static_folder::StaticFolder(folder = "front")] static_folder: PathBuf,
    #[shuttle_aws_rds::Postgres(
        local_uri = "postgres://timob:{secrets.PASSWORD}@localhost:5432/choicerank"
    )]
    pool: PgPool,
) -> shuttle_tide::ShuttleTide<Model> {
    let mut app = tide::with_state(Model { db: pool.clone() });

    app.with(tide::log::LogMiddleware::new());

    // app.at("/").get(|_| async { Ok("Hello, world!") });
    app.at("/").serve_file(static_folder.join("index.html"))?;
    app.at("/vote").nest({
        let mut api = tide::with_state(Model { db: pool });
        api.at("/").get(vote);
        api.at("/new").serve_file(static_folder.join("new.html"))?;
        api.at("/new").post(new_vote);
        api.at("/:code").get(vote);
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

    sqlx::query("INSERT INTO vote(title) values($1)")
        .bind(vote.title)
        .execute(db)
        .await?;

    Ok(tide::StatusCode::NotImplemented.into())
}

async fn vote(req: Request) -> tide::Result {
    if let Ok(Join { code }) = req.query() {
        return Ok(tide::Redirect::new(&format!("/vote/{code}")).into());
    }
    let code = req.param("code")?;
    return Ok(code.into());
}
