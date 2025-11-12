use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use anyhow::Result;
use std::path::Path;

/// Database connection pool wrapper
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection
    pub async fn new(db_path: &Path) -> Result<Self> {
        let connection_string = format!("sqlite://{}", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                connection_string.parse()?
            )
            .await?;

        // Enable WAL mode for better concurrency with Node.js
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Get reference to connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Execute a raw SQL query
    pub async fn execute(&self, query: &str) -> Result<u64> {
        let result = sqlx::query(query)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Query for a single row
    pub async fn query_one<T>(&self, query: &str) -> Result<T>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let row = sqlx::query_as::<_, T>(query)
            .fetch_one(&self.pool)
            .await?;

        Ok(row)
    }

    /// Query for multiple rows
    pub async fn query_all<T>(&self, query: &str) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let rows = sqlx::query_as::<_, T>(query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_database_connection() {
        let temp_file = NamedTempFile::new().unwrap();
        let db = Database::new(temp_file.path()).await.unwrap();

        // Test basic query
        let result = db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").await;
        assert!(result.is_ok());
    }
}
