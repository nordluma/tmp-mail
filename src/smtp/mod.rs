pub mod server;
pub mod state_machine;

#[cfg(test)]
mod tests {
    use super::state_machine::{State, StateMachine};

    #[test]
    fn test_regular_flow() {
        let mut sm = StateMachine::new("dummy");
        assert_eq!(sm.state, State::Fresh);

        sm.handle_smtp("HELO localhost").unwrap();
        assert_eq!(sm.state, State::Greeted);

        sm.handle_smtp("MAIL FROM: <local@example.com>").unwrap();
        assert!(matches!(sm.state, State::ReceivingRcpt(_)));

        sm.handle_smtp("RCPT TO: <a@localhost.com>").unwrap();
        assert!(matches!(sm.state, State::ReceivingRcpt(_)));

        sm.handle_smtp("RCPT TO: <b@localhost.com>").unwrap();
        assert!(matches!(sm.state, State::ReceivingRcpt(_)));

        sm.handle_smtp("DATA hello world\n").unwrap();
        assert!(matches!(sm.state, State::ReceivingData(_)));

        sm.handle_smtp("DATA hello world2\n").unwrap();
        assert!(matches!(sm.state, State::ReceivingData(_)));

        sm.handle_smtp("QUIT").unwrap();
        assert!(matches!(sm.state, State::Received(_)));
    }

    #[test]
    fn test_no_greeting() {
        let mut sm = StateMachine::new("dummy");
        assert_eq!(sm.state, State::Fresh);

        for command in [
            "MAIL FROM: <local@example.com>",
            "RCPT TO: <local@example.com",
            "DATA hey",
            "GARBAGE",
        ] {
            assert!(sm.handle_smtp(command).is_err());
        }
    }
}
