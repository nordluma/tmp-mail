use anyhow::{Context, Result};
use libsql_client::Statement;

use crate::Mail;

pub struct Client {
    db: libsql_client::client::Client,
}

impl Client {
    /// Create a new database client.
    ///
    /// If the `LIBSQL_CLIENT_URL` environment variable is not set, fallback to
    /// a local database.
    /// It's possible to use a remote database by setting the `LIBSQL_CLIENT_URL`
    /// environment variable.
    ///
    /// The `mail` table will be created on startup if it has not been created
    /// earlier.
    pub async fn new() -> Result<Self> {
        if std::env::var("LIBSQL_CLIENT_URL").is_err() {
            let mut db_path = std::env::temp_dir();
            db_path.push("tmp-mail.db");

            let db_path = db_path.display();
            tracing::warn!(
                "LIBSQL_CLIENT_URL not set, using default local database {}",
                db_path
            );
            std::env::set_var("LIBSQL_CLIENT_URL", format!("file://{}", db_path));
        }

        let db = libsql_client::client::Client::from_env().await?;
        db.batch({
            [
            "CREATE TABLE IF NOT EXISTS mail (date text, sender text, recipients text, data text)",
            "CREATE INDEX IF NOT EXISTS mail_date ON mail(date)",
            "CREATE INDEX IF NOT EXISTS mail_recipients ON mail(recipients)"
        ]
        })
        .await?;

        Ok(Self { db })
    }
}
