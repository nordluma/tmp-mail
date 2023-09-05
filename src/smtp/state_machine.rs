use anyhow::{Context, Result};

use crate::Mail;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Fresh,
    Greeted,
    ReceivingRcpt(Mail),
    ReceivingData(Mail),
    Received(Mail),
}

/// State machine which is capable of handling SMTP commands
/// for receiving mail.
///
/// # Usage
///
/// Use `handle_smtp` to handle a single command.
pub struct StateMachine {
    pub state: State,
    ehlo_greeting: String,
}

impl StateMachine {
    pub const START: &[u8] = b"220 tmp-mail\n";
    pub const OK: &[u8] = b"250 Ok\n";
    pub const AUTH_OK: &[u8] = b"235 Ok\n";
    pub const SEND_DATA: &[u8] = b"354 End data with <CR><LF>.<CR><LF>\n";
    pub const END: &[u8] = b"221 Bye\n";
    pub const WAIT: &[u8] = &[];

    pub fn new(domain: impl AsRef<str>) -> Self {
        let domain = domain.as_ref();
        let ehlo_greeting = format!("250-{} Hello {}\n250 AUTH PLAIN LOGIN\n", domain, domain);

        Self {
            state: State::Fresh,
            ehlo_greeting,
        }
    }

    /// Handle a single SMTP command and return a proper response
    /// # Returns
    ///
    /// Returns a response which should be sent back to the client.
    pub fn handle_smtp(&mut self, raw_msg: &str) -> Result<&[u8]> {
        tracing::trace!("Received {} in state {:?}", raw_msg, self.state);
        let mut msg = raw_msg.split_ascii_whitespace();
        let command = msg.next().context("received empty command")?.to_lowercase();
        let state = std::mem::replace(&mut self.state, State::Fresh);

        match (command.as_str(), state) {
            ("ehlo", State::Fresh) => {
                tracing::trace!("Sending AUTH info");
                self.state = State::Greeted;
                Ok(self.ehlo_greeting.as_bytes())
            }
            ("helo", State::Fresh) => {
                self.state = State::Greeted;
                Ok(StateMachine::OK)
            }
            ("noop", _) | ("help", _) | ("info", _) | ("vrfy", _) | ("expn", _) => {
                tracing::trace!("Got {}", command);
                Ok(StateMachine::OK)
            }
            ("rset", _) => {
                self.state = State::Fresh;
                Ok(StateMachine::OK)
            }
            ("auth", _) => {
                tracing::trace!("Acknowledging AUTH");
                Ok(StateMachine::AUTH_OK)
            }
            ("mail", State::Greeted) => {
                tracing::trace!("Receiving MAIL");
                let from = msg.next().context("received empty mail")?;
                let from = from
                    .strip_prefix("FROM:")
                    .context("received incorrect MAIL")?;

                tracing::debug!("FROM: {}", from);
                self.state = State::ReceivingRcpt(Mail {
                    from: from.to_string(),
                    ..Default::default()
                });

                Ok(StateMachine::OK)
            }
            ("rcpt", State::ReceivingRcpt(mut mail)) => {
                tracing::trace!("Receiving rcpt");
                let to = msg.next().context("received empty RCPT")?;
                let to = to.strip_prefix("TO:").context("received incorrect RCPT")?;

                tracing::debug!("TO: {}", to);
                mail.to.push(to.to_string());
                self.state = State::ReceivingRcpt(mail);

                Ok(StateMachine::OK)
            }
            ("data", State::ReceivingRcpt(mail)) => {
                tracing::trace!("Receiving data");
                self.state = State::ReceivingData(mail);
                Ok(StateMachine::SEND_DATA)
            }
            ("quit", State::ReceivingData(mail)) => {
                tracing::trace!(
                    "Received data: FROM: {} TO: {} DATA: {}",
                    mail.from,
                    mail.to.join(", "),
                    mail.data
                );

                self.state = State::Received(mail);
                Ok(StateMachine::END)
            }
            ("quit", _) => {
                tracing::warn!("Received quit before getting any data");
                Ok(StateMachine::END)
            }
            (_, State::ReceivingData(mut mail)) => {
                tracing::trace!("Receiving data");

                let resp = if raw_msg.ends_with("\r\n.\r\n") {
                    StateMachine::OK
                } else {
                    StateMachine::WAIT
                };

                mail.data += raw_msg;
                self.state = State::ReceivingData(mail);

                Ok(resp)
            }
            _ => anyhow::bail!(
                "Unexpected message received in state {:?}: {}",
                self.state,
                raw_msg,
            ),
        }
    }
}
