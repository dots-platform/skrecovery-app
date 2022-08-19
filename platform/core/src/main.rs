mod dec_exec;
mod client;

use client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];
    let in_files = [String::from("data1.txt"), String::from("data2.txt")];

    let cli_id = "user1";
    let app_name = "example_app";
    let func_name = "";
    let mut client = Client::new(cli_id);
    
    client.setup(node_addrs.to_vec());
    client.upload_blob(String::from("blob"), vec![vec![]]).await;
    client.exec(app_name, func_name, in_files.to_vec(), [String::from("out.txt")].to_vec()).await?;
    client.retrieve_blob(String::from("blob")).await;
    
    Ok(())
}