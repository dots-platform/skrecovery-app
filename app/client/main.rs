use std::env;

use dtrust::client::Client;

use async_trait::async_trait;

#[async_trait]
pub trait PublicKeyStorage {
    async fn upload_pk(&self, key: String, val: i64);
    async fn recover_pk(&self, key: String) -> i64;
}

#[async_trait]
impl PublicKeyStorage for Client {
    async fn upload_pk(&self, key: String, val: i64) {
        let mut upload_val = vec![];
        for _ in 0..self.node_addrs.len() {
            upload_val.push(val.to_ne_bytes().to_vec());
        }
        self.upload_blob(key, upload_val).await;
    }

    async fn recover_pk(&self, key: String) -> i64 {
        let vec_val: Vec<Vec<u8>> = self.retrieve_blob(key).await;
        let val: i64 = i64::from_ne_bytes(vec_val[0][0..8].try_into().unwrap());
        println!("recover public-key {:?}", val);
        val
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1]; 

    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];
    let in_files = [String::from("sk")];

    let cli_id = "user1";
    let app_name = "example_app";
    let func_name = "";
    let mut client = Client::new(cli_id);
    
    client.setup(node_addrs.to_vec());
    
    match &cmd[..]{
        "upload_pk" => {
            let pk: i64 = match args[2].parse() {
                Ok(n) => {
                    n
                },
                Err(_) => {
                    eprintln!("error: second argument not an integer");
                    panic!("");
                },
            };
            println!("Uploading pk {} for user {}", pk, cli_id);
            client.upload_pk(String::from(cli_id), pk).await;
            
        }  
        "recover_pk" => {
            println!("Recovering pk");
            let id: String = match args[2].parse() {
                Ok(n) => {
                    n
                },
                Err(_) => {
                    eprintln!("error: second argument not a string");
                    panic!("");
                },
            };
            //client.exec(app_name, func_name, in_files.to_vec(), [String::from("out")].to_vec()).await?;
            client.recover_pk(id).await;
        }

        _=> println!("Missing/wrong arguments")

        // "upload_pk" => 

    };

    
    Ok(())
}