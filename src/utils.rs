use std::pin::Pin;

use sqlx::PgConnection;

use crate::error::Result;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub async fn with_tx<F, T>(conn: &mut PgConnection, f: F) -> Result<T>
where
    F: AsyncFnOnce(&mut PgConnection) -> Result<T>,
{
    let mut tx = sqlx::Connection::begin(conn).await?;
    let result = f(tx.as_mut()).await;

    match result {
        Ok(_) => tx.commit().await?,
        Err(_) => tx.rollback().await?,
    }

    result
}
