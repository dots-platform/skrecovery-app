use std::env;
use std::fs::File;

use dtrust::client::Client;

use async_trait::async_trait;
use yaml_rust::yaml::{Hash, Yaml};
use yaml_rust::YamlLoader;
use clap::Parser;

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
        //println!("recover public-key {:?}", key);
        println!("{:?}", key);
        key
    }
}


#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
   /// Name of the person to greet
   #[clap(short, long, value_parser)]
   client_id: String,

   /// Number of times to greet
   #[clap(parse(from_os_str))]
   config: std::path::PathBuf,
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cmd = &args[1]; 

    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];

    let cli_id = "user1";
    let mut client = Client::new(cli_id);
    
    client.setup(node_addrs.to_vec());

    let conf_path = &args[2].parse()?;
    
    let mut file = File::open(conf_path).expect("Unable to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read file");
    let conf = YamlLoader::load_from_str(&contents).unwrap();
    
    let cli_id = &args[3].parse()?;


    // let mut file = File::open(file).expect("Unable to open file");
    // let mut contents = String::new();

    // file.read_to_string(&mut contents)
    //     .expect("Unable to read file");

    // let conf = YamlLoader::load_from_str(&contents).unwrap();
    
    // match &cmd[..]{
    //     "upload_pk" => {
    //         let id: String = match args[2].parse() {
    //             Ok(s) => {
    //                 s
    //             },
    //             Err(_) => {
    //                 eprintln!("error: user-id not a string");
    //                 panic!("");
    //             },
    //         };
    //         let pk: String = match args[3].parse() {
    //             Ok(s) => {
    //                 s
    //             },
    //             Err(_) => {
    //                 eprintln!("error: pk not a string");
    //                 panic!("");
    //             },
    //         };
    //         println!("Uploading pk {} for user {}", pk, id);
    //         client.upload_pk(String::from(id), pk).await;
            
    //     }  
    //     "recover_pk" => {
    //         //println!("Recovering pk");
    //         let id: String = match args[2].parse() {
    //             Ok(n) => {
    //                 n
    //             },
    //             Err(_) => {
    //                 eprintln!("error: second argument not a string");
    //                 panic!("");
    //             },
    //         };
    //         client.recover_pk(id).await;
    //     }

    //     _=> println!("Missing/wrong arguments")

    // };

    
    Ok(())
}