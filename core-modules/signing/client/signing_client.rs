use core::num;
use std::env;

use dtrust::client::Client;

use async_trait::async_trait;

#[async_trait]
pub trait ThresholdSigning {
    async fn upload_params(&self, id: String, num_threshold: String, num_parties: String);
}

#[async_trait]
impl ThresholdSigning for Client {
    async fn upload_params(&self, id: String, num_threshold: String, num_parties: String) {
        let params = [
            num_parties.as_bytes().to_vec(),
            (" ").as_bytes().to_vec(),
            num_threshold.as_bytes().to_vec(),
        ]
        .concat();
        let upload_val = vec![params; self.node_addrs.len()];
        self.upload_blob(id.clone(), upload_val).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let func_name = args[1].clone();
    let num_parties = args[2].parse().unwrap();
    let num_threshold = args[3].parse().unwrap();

    if func_name == "keygen" {
        let node_addrs = [
            "http://127.0.0.1:50051",
            "http://127.0.0.1:50052",
            "http://127.0.0.1:50053",
        ];

        let in_files = [String::from("user1")]; // TODO: Fill in with signing input file message to sign
        let out_files = [String::from("keys.json")]; // TODO: Fill in with keygen output files

        let cli_id = "user1";
        let app_name = "rust_app";
        let mut client = Client::new(cli_id);

        client.setup(node_addrs.to_vec());

        client
            .upload_params(String::from(cli_id), num_threshold, num_parties)
            .await;

        client
            .exec(app_name, "keygen", in_files.to_vec(), out_files.to_vec())
            .await?;
    } else if func_name == "signing" {
        let node_addrs = [
            "http://127.0.0.1:50051",
            "http://127.0.0.1:50052",
            "http://127.0.0.1:50053"
        ];

        let in_files = [String::from("user1"), String::from("keys.json")]; // TODO: Fill in with signing input file message to sign
        let out_files = [String::from("signature")]; // TODO: Fill in with keygen output files

        let cli_id = "user1";
        let app_name = "rust_app";
        let mut client = Client::new(cli_id);

        client.setup(node_addrs.to_vec());

        client
            .upload_params(String::from(cli_id), num_threshold, num_parties)
            .await;

        client
            .exec(app_name, "signing", in_files.to_vec(), out_files.to_vec())
            .await?;
    }

    Ok(())
}
