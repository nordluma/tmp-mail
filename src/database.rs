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
        db.batch([
            "CREATE TABLE IF NOT EXISTS mail (date text, sender text, recipients text, data text)",
            "CREATE INDEX IF NOT EXISTS mail_date ON mail(date)",
            "CREATE INDEX IF NOT EXISTS mail_recipients ON mail(recipients)",
        ])
        .await?;

        Ok(Self { db })
    }

    /// Replicate received mail to the database
    pub async fn replicate(&self, mail: Mail) -> Result<()> {
        let now = chrono::offset::Utc::now()
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();

        self.db
            .execute(Statement::with_args(
                "INSERT INTO mail VALUES (?, ?, ?, ?)",
                libsql_client::args!(now, mail.from, mail.to.join(", "), mail.data),
            ))
            .await
            .map(|_| ())
    }

    /// Delete old mails
    pub async fn delete_old_mail(&self) -> Result<()> {
        let now = chrono::offset::Utc::now();
        let a_week_ago = now - chrono::Duration::days(7);
        let a_week_ago = &a_week_ago.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        tracing::debug!("Deleting old mail from before {}", a_week_ago);

        let count: i64 = i64::try_from(
            self.db
                .execute(Statement::with_args(
                    "SELECT COUNT(*) FROM mail WHERE date < ?",
                    libsql_client::args!(a_week_ago),
                ))
                .await?
                .rows
                .first()
                .context("No rows returned from a COUNT(*) query")?
                .values
                .first()
                .context("NO values returned from a COUNT(*) query")?,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        tracing::debug!("Found {} old mail", count);

        self.db
            .execute(Statement::with_args(
                "DELETE FROM mail WHERE date < ?",
                libsql_client::args!(a_week_ago),
            ))
            .await
            .ok();
        Ok(())
    }
}
