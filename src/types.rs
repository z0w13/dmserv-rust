use std::sync::Arc;

use crate::modules::stats;

#[derive(Debug)]
pub(crate) struct Data {
    pub(crate) db: sqlx::PgPool,
    pub(crate) stats: stats::Stats,
}

impl Data {
    pub(crate) fn new(db: sqlx::PgPool) -> Self {
        Self {
            db,
            stats: stats::Stats::new(),
        }
    }
}

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Context<'a> = poise::Context<'a, Arc<Data>, Error>;
