use std::path::PathBuf;

use serde::Deserialize;
use tide::Request;

#[shuttle_runtime::main]
async fn tide(
    #[shuttle_static_folder::StaticFolder(folder = "front")] static_folder: PathBuf,
) -> shuttle_tide::ShuttleTide<()> {
    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    // app.at("/").get(|_| async { Ok("Hello, world!") });
    app.at("/").serve_file(static_folder.join("index.html"))?;
    app.at("/vote").nest({
        let mut api = tide::new();
        api.at("/").get(vote);
        api.at("/new").patch(|_| async { Ok("<h3>hello</h3>") });
        api.at("/new").serve_file(static_folder.join("new.html"))?;
        api.at("/:code").get(vote);
        api
    });

    Ok(app.into())
}

#[derive(Deserialize)]
struct Vote {
    code: String,
}

async fn vote(req: Request<()>) -> tide::Result {
    if let Ok(Vote { code }) = req.query() {
        return Ok(tide::Redirect::new(&format!("/vote/{code}")).into());
    }
    let code = req.param("code")?;
    return Ok(code.into());
}
