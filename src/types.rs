use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct Data {
    pub(crate) db: sqlx::PgPool,
}

impl Data {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }
}

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Context<'a> = poise::Context<'a, Arc<Data>, Error>;
