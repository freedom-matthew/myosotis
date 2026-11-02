use std::error::Error;
use std::path::Path;

use myosotis_core::{Card, CardRepo, RepoError};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub struct SqliteCardRepo {
    pool: SqlitePool,
}

impl SqliteCardRepo {
    /// Opens (or creates) the database file and brings its schema up to date.
    pub async fn open(path: &Path) -> Result<Self, RepoError> {
        let options: SqliteConnectOptions = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool: SqlitePool = SqlitePool::connect_with(options)
            .await
            .map_err(repo_error)?;
        Self::migrated(pool).await
    }

    /// Fresh, empty database that lives only as long as the repo. Meant for tests.
    pub async fn in_memory() -> Result<Self, RepoError> {
        // Every connection to `:memory:` is its own database, so keep exactly one, forever.
        let pool: SqlitePool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect("sqlite::memory:")
            .await
            .map_err(repo_error)?;
        Self::migrated(pool).await
    }

    async fn migrated(pool: SqlitePool) -> Result<Self, RepoError> {
        sqlx::migrate!().run(&pool).await.map_err(repo_error)?;
        Ok(Self { pool })
    }

    pub async fn insert(&self, card: &Card) -> Result<(), RepoError> {
        sqlx::query!(
            "INSERT INTO cards (question, answer) VALUES (?, ?)",
            card.question,
            card.answer
        )
        .execute(&self.pool)
        .await
        .map_err(repo_error)?;
        Ok(())
    }
}

impl CardRepo for SqliteCardRepo {
    async fn list(&self) -> Result<Vec<Card>, RepoError> {
        sqlx::query_as!(Card, "SELECT question, answer FROM cards ORDER BY id")
            .fetch_all(&self.pool)
            .await
            .map_err(repo_error)
    }
}

fn repo_error(error: impl Error + Send + Sync + 'static) -> RepoError {
    RepoError(Box::new(error))
}
