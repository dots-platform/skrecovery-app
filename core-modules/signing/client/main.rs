use std::env;

use dtrust::client::Client;

use async_trait::async_trait;

#[async_trait]
pub trait ThresholdSigning {
    async fn upload_params(
        &self,
        id: String,
        num_threshold: String,
        num_parties: String,
        active_parties: String,
    );
    async fn upload_message(&self, id: String, message: String);
}

#[async_trait]
impl ThresholdSigning for Client {
    async fn upload_params(
        &self,
        id: String,
        num_threshold: String,
        num_parties: String,
        active_parties: String,
    ) {
        let params = [
            num_parties.as_bytes().to_vec(),
            (" ").as_bytes().to_vec(),
            num_threshold.as_bytes().to_vec(),
            (" ").as_bytes().to_vec(),
            active_parties.as_bytes().to_vec(),
        ]
        .concat();
        let upload_val = vec![params; self.node_addrs.len()];
        self.upload_blob(id.clone(), upload_val).await;
    }
    async fn upload_message(&self, id: String, message: String) {
        let upload_val = vec![message.as_bytes().to_vec(); self.node_addrs.len()];
        self.upload_blob(id.clone(), upload_val).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1];
    
    let node_addrs = [
        "http://127.0.0.1:50051",
        "http://127.0.0.1:50052",
        "http://127.0.0.1:50053",
    ];

    let cli_id = "user1";
    let app_name = "rust_app";
    let mut client = Client::new(cli_id);
    client.setup(node_addrs.to_vec());

    let num_parties: String = match args[2].parse() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("error: num_parties not a string");
            panic!("");
        }
    };

    let num_threshold: String = match args[3].parse() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("error: num_threshold not a string");
            panic!("");
        }
    };

    match &cmd[..] {
        "keygen" => {
            client
                .upload_params(
                    String::from(cli_id),
                    num_threshold,
                    num_parties,
                    "".to_string(),
                )
                .await;

            let in_files = [String::from("user1")];
            let out_files = [String::from("key.json")];

            client
                .exec(app_name, "keygen", in_files.to_vec(), out_files.to_vec())
                .await?;
        }
        "sign" => {
            let active_parties: String = match args[4].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("error: active_parties not a string");
                    panic!("");
                }
            };

            let message: String = match args[5].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("error: message not a string");
                    panic!("");
                }
            };

            client
                .upload_params(
                    String::from(cli_id),
                    num_threshold,
                    num_parties,
                    active_parties,
                )
                .await;

            let in_files = [
                String::from("user1"),
                String::from("key.json"),
                String::from("message"),
            ];
            let out_files = [String::from("signature.json")];

            client
                .upload_message(String::from("message"), message)
                .await;

            client
                .exec(app_name, "signing", in_files.to_vec(), out_files.to_vec())
                .await?;
        }

        _ => println!("Missing/wrong arguments"),
    };

    Ok(())
}
