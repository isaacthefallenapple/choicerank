use std::collections::HashMap;

use askama::Template;
use serde::{de::Deserializer, Deserialize};
use sqlx::FromRow;
use tide::sse;

use crate::Request;

pub const CHOICE_SEPARATOR: char = '\x1F';

fn id(req: &Request) -> tide::Result<i32> {
    let id: i32 = req.param("code")?.parse()?;

    Ok(id)
}

pub async fn get(req: Request) -> tide::Result {
    let id = id(&req)?;

    let ballot = select_ballot(req.state().db(), id).await?;

    Ok(ballot.into())
}

pub async fn submit(mut req: Request) -> tide::Result {
    let id = id(&req)?;

    let ranking: Ranking = req.body_form().await?;

    req.state()
        .send_to(id, "voter", &format!("<li>{}</li>", &ranking.name))
        .await;

    let _ = sqlx::query!(
        r#"
INSERT INTO ranking (id, name, ranking)
VALUES ($1, $2, $3)
        "#,
        id,
        ranking.name,
        ranking.ranking,
    )
    .execute(req.state().db())
    .await?;

    let results = format!("/vote/{id}/results");

    let mut response: tide::Response = tide::StatusCode::Created.into();
    set_client_side_redirect(&mut response, results);

    Ok(response)
}

pub async fn new(mut req: Request) -> tide::Result {
    let ballot: Ballot = req.body_form().await?;

    let redirect_target = req
        .header("location")
        .map(|s| s.as_str())
        .unwrap_or("ballot");

    let rec = sqlx::query!(
        r#"
INSERT INTO ballot (title, choices, max_choices, open)
VALUES ($1, $2, $3, $4)
RETURNING id"#,
        ballot.title,
        ballot.choices.0,
        ballot.max_choices,
        true,
    )
    .fetch_one(req.state().db())
    .await?;

    let id = rec.id;

    let redirect = format!("/vote/{id}/{redirect_target}");

    let mut response: tide::Response = ballot.into();
    response.set_status(tide::StatusCode::Created);
    response.insert_header("HX-Push-Url", redirect);

    Ok(response)
}

pub async fn results(req: Request) -> tide::Result {
    let id = id(&req)?;

    struct Row {
        name: Option<String>,
        ranking: String,
    }

    let rows = sqlx::query_as!(
        Row,
        r#"
select name, ranking
from ranking
where id = $1
        "#,
        id
    )
    .fetch_all(req.state().db())
    .await?;

    let rankings: HashMap<String, String> = rows
        .into_iter()
        .map(|row| (row.name.unwrap_or_default(), row.ranking))
        .collect();

    let results = Results { rankings };

    let mut res: tide::Response = results.into();

    res.set_status(tide::StatusCode::Ok);

    Ok(res)
}

pub async fn live(req: Request, sender: sse::Sender) -> tide::Result<()> {
    let id = id(&req)?;
    req.state().register_sse_sender(id, sender).await;

    Ok(())
}

pub async fn join(req: Request) -> tide::Result {
    #[derive(Deserialize)]
    struct Join {
        code: i32,
    }

    let Join { code } = req.query()?;

    let ballot: Ballot = match select_ballot(req.state().db(), code).await {
        Ok(ballot) => ballot.into(),
        Err(sqlx::Error::RowNotFound) => return Ok(tide::StatusCode::NotFound.into()),
        Err(err) => return Err(err.into()),
    };

    let mut resp: tide::Response = ballot.into();
    resp.insert_header("HX-Push-Url", format!("/vote/{code}/ballot"));

    Ok(resp)
}

#[derive(Debug, Template)]
#[template(path = "results.html")]
struct Results {
    rankings: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Template, FromRow)]
#[template(path = "ballot.html")]
pub struct Ballot {
    title: String,
    choices: Choices,
    #[serde(rename(deserialize = "max-choices"))]
    max_choices: i32,
    // #[serde(default)]
    // anonymous: bool,
    // #[serde(deserialize_with = "deserialize_password")]
    // password: Option<String>,
}

#[derive(Debug, Deserialize, FromRow)]
struct Ranking {
    name: String,
    ranking: String,
}

// impl Ballot {
//     pub fn new(title: String, choices: String, max_choices: i32) -> Self {
//         Self {
//             title,
//             choices: Choices::new(choices),
//             max_choices,
//         }
//     }
//
//     pub fn title(&self) -> &str {
//         &self.title
//     }
//
//     pub fn choices(&self) -> &Choices {
//         &self.choices
//     }
//
//     pub fn max_choices(&self) -> i32 {
//         self.max_choices
//     }
// }

#[derive(Debug, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct Choices(String);

impl Choices {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.into_iter()
    }

    pub fn collect(&self) -> Vec<&str> {
        self.iter().collect()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl<'a> IntoIterator for &'a Choices {
    type Item = &'a str;
    type IntoIter = std::str::Split<'a, char>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.split(CHOICE_SEPARATOR)
    }
}

impl AsRef<[u8]> for Choices {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<str> for Choices {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn set_client_side_redirect(
    resp: &mut tide::Response,
    location: impl tide::http::headers::ToHeaderValues,
) {
    resp.insert_header("HX-Redirect", location)
}

pub async fn select_ballot(pool: &sqlx::PgPool, id: i32) -> sqlx::Result<Ballot> {
    let ballot = sqlx::query_as!(
        Ballot,
        r#"
SELECT title, choices as "choices: _", max_choices
FROM ballot
WHERE id = $1 AND open = true
        "#,
        id
    )
    .fetch_one(pool)
    .await?;

    Ok(ballot)
}

fn _deserialize_password<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
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
            Ok(if v.is_empty() {
                None
            } else {
                Some(v.to_string())
            })
        }
    }

    deserializer.deserialize_str(V)
}
