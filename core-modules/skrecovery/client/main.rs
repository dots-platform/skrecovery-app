use std::env;

use dtrust::client::Client;

use async_trait::async_trait;

#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    async fn upload_sk_and_pwd(&self, sk: String, pwd: String);
    async fn recover_sk(&self, pwd_guess: String) -> String;
}

#[async_trait]
impl SecretKeyRecoverable for Client {
    async fn upload_sk_and_pwd(&self, sk: String, pwd: String) {
        todo!();
    }

    async fn recover_sk(&self, pwd_guess: String) -> String {
        todo!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];

    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];

    let cli_id = "user1";
    let mut client = Client::new(cli_id);

    client.setup(node_addrs.to_vec());

    match &cmd[..]{
        "upload_sk_and_pwd" => {
        }
        "recover_sk" => {
        }

        _=> println!("Missing/wrong arguments")
    };
    Ok(())
}