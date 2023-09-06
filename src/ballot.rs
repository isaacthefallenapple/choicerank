use askama::Template;
use serde::{de::Deserializer, Deserialize};
use sqlx::FromRow;
use tide;
use tide::sse;

use crate::Request;

pub const CHOICE_SEPARATOR: char = '\x1F';

fn id(req: &Request) -> tide::Result<i32> {
    let id: i32 = req.param("code")?.parse()?;

    Ok(id)
}

pub async fn get(req: Request) -> tide::Result {
    let id = id(&req)?;

    let ballot = sqlx::query_as!(
        Ballot,
        r#"select title, choices as "choices: _", max_choices from ballot where id = $1"#,
        id
    )
    .fetch_one(req.state().db())
    .await?;

    Ok(ballot.into())
}

pub async fn post(mut req: Request) -> tide::Result {
    let id = id(&req)?;

    let ranking: Ranking = req.body_form().await?;

    req.state()
        .send_to(id, "voter", &format!("<li>{}</li>", &ranking.name))
        .await;

    let _ = sqlx::query!(
        r#"insert into ranking (id, name, ranking) values ($1, $2, $3)"#,
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

    let rec = sqlx::query!(
        r#"insert into ballot (title, choices, max_choices) values ($1, $2, $3) returning id"#,
        ballot.title,
        ballot.choices.0,
        ballot.max_choices
    )
    .fetch_one(req.state().db())
    .await?;

    let id = rec.id;

    let redirect = format!("/vote/{id}/ballot");

    let mut response: tide::Response = tide::StatusCode::Created.into();
    set_client_side_redirect(&mut response, redirect);

    Ok(response)
}

pub async fn results(_req: Request) -> tide::Result {
    let mut res = tide::Response::new(tide::StatusCode::Ok);
    let body: tide::Body = dbg!(tide::Body::from_file("front/results.html").await)?;
    res.set_body(body);

    Ok(res)
}

pub async fn live(req: Request, sender: sse::Sender) -> tide::Result<()> {
    let id = id(&req)?;
    req.state().register_sse_sender(id, sender).await;

    Ok(())
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
