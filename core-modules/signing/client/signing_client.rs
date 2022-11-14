use std::env;

use dtrust::client::Client;

use async_trait::async_trait;

#[async_trait]
pub trait ThresholdSigning {
    async fn upload_params(&self, id: String, num_threshold: String, num_parties: String, active_parties: String);
    async fn upload_message(&self, id: String, message: String);
}

#[async_trait]
impl ThresholdSigning for Client {
    async fn upload_params(&self, id: String, num_threshold: String, num_parties: String, active_parties: String) {
        let params = [
            num_parties.as_bytes().to_vec(),
            (" ").as_bytes().to_vec(),
            num_threshold.as_bytes().to_vec(),
            (" ").as_bytes().to_vec(),
            active_parties.as_bytes().to_vec()
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
    let func_name = args[1].clone();
    let num_parties = args[2].parse().unwrap();
    let num_threshold = args[3].parse().unwrap();

    let node_addrs = [
        "http://127.0.0.1:50051",
        "http://127.0.0.1:50052",
        "http://127.0.0.1:50053"
    ];

    let cli_id = "user1";
    let app_name = "rust_app";
    let mut client = Client::new(cli_id);

    client.setup(node_addrs.to_vec());

    if func_name == "keygen" {
        client
        .upload_params(String::from(cli_id), num_threshold, num_parties, "".to_string())
        .await;

        let in_files = [String::from("user1")]; 
        let out_files = [String::from("key.json")]; 

        client
            .exec(app_name, "keygen", in_files.to_vec(), out_files.to_vec())
            .await?;
    } else if func_name == "signing" {
        let active_parties = args[4].parse().unwrap();
        let message = args[5].parse().unwrap();

        client
        .upload_params(String::from(cli_id), num_threshold, num_parties, active_parties)
        .await;

        let in_files = [String::from("user1"), String::from("key.json"), String::from("message")]; 
        let out_files = [String::from("signature.json")]; 

        client.upload_message(String::from("message"), message).await;

        client
            .exec(app_name, "signing", in_files.to_vec(), out_files.to_vec())
            .await?;
    }

    Ok(())
}
