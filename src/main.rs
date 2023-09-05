use std::env;

use anyhow::{Context, Result};
use tokio::net::TcpListener;

use tmp_mail::smtp::server;

/// Helper function for cleaning old mails from the database
fn clean_db(period: tokio::time::Duration) {
    std::thread::spawn(move || -> Result<()> {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .enable_io()
            .build()
            .context("failed to build async runtime")?
            .block_on(async move {
                let local = tokio::task::LocalSet::new();
                local.spawn_local(async move {
                    let db = match tmp_mail::database::Client::new().await {
                        Ok(db) => db,
                        Err(e) => {
                            tracing::error!("Failed to connect to database: {}", e);
                            return;
                        }
                    };

                    let mut interval = tokio::time::interval(period);
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                    loop {
                        interval.tick().await;

                        if let Err(e) = db.delete_old_mail().await {
                            tracing::error!("Failed to delete old mail: {}", e);
                        }
                    }
                });
                local.await;
            });
        Ok(())
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:2525".to_string());

    let domain = &env::args()
        .nth(2)
        // Not sure about this one
        .unwrap_or_else(|| "smtp.idont.date".to_string());

    tracing::info!("tmp-mail server for {} started", domain);

    let listener = TcpListener::bind(&addr).await?;

    // Task for deleting old mails
    clean_db(tokio::time::Duration::from_secs(60));

    loop {
        let (stream, addr) = listener.accept().await?;
        tracing::info!("Accepted a connection from {}", addr);

        tokio::task::LocalSet::new()
            .run_until(async move {
                let smtp = server::Server::new(domain, stream).await?;
                smtp.serve().await
            })
            .await
            .ok();
    }
}
