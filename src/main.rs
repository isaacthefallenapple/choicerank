use std::path::PathBuf;

use sqlx::PgPool;
pub use state::{Request, State};
use tide::sse;

mod ballot;
mod state;

#[shuttle_runtime::main]
async fn tide(
    #[shuttle_static_folder::StaticFolder(folder = "front")] static_folder: PathBuf,
    #[shuttle_aws_rds::Postgres(
        local_uri = "postgres://timob:{secrets.PASSWORD}@localhost:5432/choicerank"
    )]
    pool: PgPool,
) -> shuttle_tide::ShuttleTide<State> {
    let model = State::new(pool.clone());
    let mut app = tide::with_state(model.clone());

    app.with(tide::log::LogMiddleware::new());

    // app.at("/").get(|_| async { Ok("Hello, world!") });
    app.at("/front/assets")
        .serve_dir(static_folder.join("assets"))?;
    app.at("/").serve_file(static_folder.join("index.html"))?;
    app.at("/new")
        .post(ballot::new)
        .serve_file(static_folder.join("new.html"))?;

    app.at("/vote/:code/").nest({
        let mut api = tide::with_state(model.clone());
        api.at("ballot").get(ballot::get).post(ballot::post);
        api.at("results").get(ballot::results);
        api.at("results/live").get(sse::endpoint(ballot::live));
        api
    });

    Ok(app.into())
}
