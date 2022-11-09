use std::env;

use dtrust::client::Client;

use async_trait::async_trait;

#[async_trait]
pub trait SecretKeyRecoverable {
    // Encrypt the secret key?
    async fn upload_sk_and_pwd(&self, id: String, sk: String, pwd: String);
    async fn recover_sk(&self, id: String, pwd_guess: String) -> String;
}

#[async_trait]
impl SecretKeyRecoverable for Client {
    async fn upload_sk_and_pwd(&self, id: String, sk: String, pwd: String) {
        todo!();
    }

    async fn recover_sk(&self, id: String, pwd_guess: String) -> String {
        todo!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd =args[1]; &args[1];

    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];

    let cli_id = "user1";
    let mut client = Client::new(cli_id);

    client.setup(node_addrs.to_vec());

    match &cmd[..]{
        "upload_sk_and_pwd" => {
            let id: String = match args[2].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: user-id not a string");
                    panic!("");
                },
            };
            let sk: String = match args[3].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: sk not a string");
                    panic!("");
                },
            };
            let pwd: String = match args[4].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: pwd not a string");
                    panic!("");
                },
            };
            println!("Uploading sk {}, pwd {} for user {}", sk, pwd, id);
            client.upload_sk_and_pwd(String::from(id), sk, pwd).await;
        }
        "recover_sk" => {
            let id: String = match args[2].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: user-id not a string");
                    panic!("");
                },
            };
            let pwd_guess: String = match args[3].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: pwd guess not a string");
                    panic!("");
                },
            };
            println!("Recovering sk with pwd guess {}, for user {}", pwd_guess, id);
            client.recover_sk(String::from(id), pwd_guess).await;
        }

        _ => println!("Missing/wrong arguments")
    };
    Ok(())
}