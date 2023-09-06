use sqlx::PgPool;

pub type Request = tide::Request<State>;

#[derive(Clone, Debug)]
pub struct State {
    db: PgPool,
}

impl State {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &PgPool {
        &self.db
    }
}
