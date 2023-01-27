use std::env;

use dtrust::client::Client;

use async_trait::async_trait;

#[async_trait]
pub trait PublicKeyStorage {
    async fn upload_pk(&self, id: String, key: String);
    async fn recover_pk(&self, id: String) -> String;
}

#[async_trait]
impl PublicKeyStorage for Client {
    async fn upload_pk(&self, id: String, key: String) {
        let upload_val = vec![key.as_bytes().to_vec(); self.node_addrs.len()];
        self.upload_blob(id, upload_val).await;
    }

    async fn recover_pk(&self, id: String) -> String {
        let vec_val: Vec<Vec<u8>> = self.retrieve_blob(id).await;
        for i in 0..self.node_addrs.len() {
            if vec_val[i] != vec_val[0] {
                panic!("Not valid public-key");
            }
        }
        let key = match String::from_utf8(vec_val[0].clone()) {
        Ok(v) => v,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    };
        println!("recovered public-key: {:?}", key);
        key
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];

    let use_tls: bool = true;
    let node_addrs =
        if use_tls == true {
            ["https://node1.test:50051", "https://node2.test:50052"]
        } else {
            ["http://127.0.0.1:50051", "http://127.0.0.1:50052"]
        };
    let rootca_certpath =
        if use_tls == true {
            Some("tls_certs/myCA.pem")
        } else {
            None
        };

    let cli_id = "user1";
    let mut client = Client::new(cli_id);

    client.setup(node_addrs.to_vec(), rootca_certpath);

    match &cmd[..]{
        "upload_pk" => {
            let id: String = match args[2].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: user-id not a string");
                    panic!("");
                },
            };
            let pk: String = match args[3].parse() {
                Ok(s) => {
                    s
                },
                Err(_) => {
                    eprintln!("error: pk not a string");
                    panic!("");
                },
            };
            println!("Uploading pk {} for user {}", pk, id);
            client.upload_pk(String::from(id), pk).await;

        }
        "recover_pk" => {
            //println!("Recovering pk");
            let id: String = match args[2].parse() {
                Ok(n) => {
                    n
                },
                Err(_) => {
                    eprintln!("error: second argument not a string");
                    panic!("");
                },
            };
            client.recover_pk(id).await;
        }

        _=> println!("Missing/wrong arguments")
    };
    Ok(())
}