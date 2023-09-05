pub mod database;
pub mod smtp;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Mail {
    pub from: String,
    pub to: Vec<String>,
    pub data: String,
}
