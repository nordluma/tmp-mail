use std::sync::Arc;

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

use crate::database;

use super::state_machine::StateMachine;

/// SMTP server
///
/// Handles user connections and replicates received messages to the database
pub struct Server {
    stream: tokio::net::TcpStream,
    state_machine: StateMachine,
    db: Arc<Mutex<database::Client>>,
}

impl Server {
    /// Create a new server from connected stream
    pub async fn new<T: AsRef<str>>(domain: T, stream: tokio::net::TcpStream) -> Result<Self> {
        Ok(Self {
            stream,
            state_machine: StateMachine::new(domain),
            db: Arc::new(Mutex::new(database::Client::new().await?)),
        })
    }

    /// Run the server loop
    ///
    /// Accept and handle SMTP commands
    pub async fn serve(mut self) -> Result<()> {
        self.greet().await?;

        let mut buf = vec![0; 65536];

        loop {
            let n = self.stream.read(&mut buf).await?;

            if n == 0 {
                tracing::info!("Received EOF");
                self.state_machine.handle_smtp("quit").ok();
                break;
            }

            let msg = std::str::from_utf8(&buf[0..n])?;
            let response = self.state_machine.handle_smtp(msg)?;

            if response != StateMachine::WAIT {
                self.stream.write_all(response).await?;
            } else {
                tracing::debug!("Not responding, awaiting more data");
            }

            if response == StateMachine::END {
                break;
            }
        }

        match self.state_machine.state {
            super::state_machine::State::ReceivingData(mail) => {
                tracing::info!("Received EOF before receiving QUIT");
                self.db.lock().await.replicate(mail).await?;
            }
            super::state_machine::State::Received(mail) => {
                self.db.lock().await.replicate(mail).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Send initial SMTP greeting
    async fn greet(&mut self) -> Result<()> {
        self.stream
            .write_all(StateMachine::START)
            .await
            .map_err(|e| e.into())
    }
}
