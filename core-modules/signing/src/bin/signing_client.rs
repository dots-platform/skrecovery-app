use dtrust::client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052", "http://127.0.0.1:50053"];
    let in_files = [String::from("in")]; // TODO: Fill in with signing input file message to sign
    let out_files = [String::from("out")]; // TODO: Fill in with keygen output files

    let cli_id = "user1";
    let app_name = "rust_app";
    let mut client = Client::new(cli_id);
    
    client.setup(node_addrs.to_vec());
    client.exec(app_name, "keygen", in_files.to_vec(), out_files.to_vec()).await?;
    client.exec(app_name, "signing", in_files.to_vec(), out_files.to_vec()).await?;
    Ok(())
}