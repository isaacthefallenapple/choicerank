use crate::Request;

pub async fn get(_req: Request) -> tide::Result {
    let mut res = tide::Response::new(tide::StatusCode::Ok);
    let body: tide::Body = dbg!(tide::Body::from_file("front/results.html").await)?;
    res.set_body(body);

    Ok(res)
}
