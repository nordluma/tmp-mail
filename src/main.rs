fn main() {
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Mail {
    pub from: String,
    pub to: Vec<String>,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Fresh,
    Greeted,
    ReceivingRcpt(Mail),
    ReceivingData(Mail),
    Received(Mail),
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
}
