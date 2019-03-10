use actix::prelude::*;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

/// This is db executor actor. We are going to run 3 of them in parallel.
pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

impl DbExecutor {
    pub fn conn(
        &self,
    ) -> diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>> {
        self.0.get().expect("error getting connection to postgres")
    }
}
