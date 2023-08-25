use std::{collections::HashMap, fmt::format, path::PathBuf};

use serde::Deserialize;
use tide::Request;

mod vote;

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

#[derive(Debug, Deserialize)]
struct Vote {
    title: String,
    #[serde(deserialize_with = "deserialize_choices")]
    choices: Vec<String>,
    #[serde(rename(deserialize = "max-choices"))]
    max_choices: usize,
    #[serde(default)]
    anonymous: bool,
    #[serde(deserialize_with = "deserialize_password")]
    password: Option<String>,
}

use serde::Deserializer;

fn deserialize_password<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> serde::de::Visitor<'de> for V {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok((!v.is_empty()).then(|| v.to_string()))
        }
    }

    deserializer.deserialize_str(V)
}

fn deserialize_choices<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> serde::de::Visitor<'de> for V {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence of strings separated by \"|||\"")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v.split("|||")
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
                .collect())
        }
    }

    deserializer.deserialize_str(V)
}

async fn new_vote(mut req: Request<()>) -> tide::Result {
    let vote: Vote = dbg!(req.body_form().await?);
    Ok(tide::StatusCode::NotImplemented.into())
}

async fn vote(req: Request<()>) -> tide::Result {
    if let Ok(Join { code }) = req.query() {
        return Ok(tide::Redirect::new(&format!("/vote/{code}")).into());
    }
    let code = req.param("code")?;
    return Ok(code.into());
}
