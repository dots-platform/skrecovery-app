# Developing your decentralized application

## Workflow of a decentralized application
The dtrust platform consists of a client and multiple decentralized nodes (servers). The client interacts with the nodes through gRPC requests.

A typical decentralized application consists of the following steps:
1. A client performs some local pre-computation, and uploads the results of the pre-computations to the servers. The pre-computation results are stored as files on the servers.
2. The client initiates a request to execute a decentralized app remotely on the servers. The servers take the input files as specified by the client, jointly executes the decentralized app, and stores the results to output files specified by the client. 
3. The client downloads the results from the servers, and performs some post-computation on the results locally.

The above workflow only provides a general guideline for developing decentralized applications. A decentralized application does not need to follow these steps exactly. Each step is separate and composable from each other, and can be optional depending on the specific application you are building. For example, an app that does secret-key recovery does not have any server side computations, so it can skip step 2.  

### Developing Client-side Computations (Step 1, Step 3)
The dtrust platform provides the `upload_blob` and `retrive_blob` functions to upload and retrieve any data as files.

Before uploading or downloading any data, the client first needs to specify which nodes it will connect to:
```rust
let cli_id = "user1";
let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];
let mut client = Client::new(cli_id);
client.setup(node_addrs.to_vec());
```

The `upload_blob` function will takes a string as key, and a list of bytes as values. It will upload the i'th bytes in the list to the i'th node, and store it in a file indexed by the key.

The `retrieve_blob` function will retrieve the previously uploaded files indexed by the key.


Here is an example client function that uploads a public key to all servers using `upload_blob` and recover them through `retrieve_blob`.
```rust
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
    println!("recover public-key {:?}", key);
    key
}
```

### Developing a decentralized App on the servers (Step 2)
A decentralized app on the server is an executable that the servers jointly executes. The client invokes the executable through gRPC requests, and specify the names of the input files and output files. The dtrust platform first open these files and setups network connections. Then, the dtrust platform forks a subprocess to execute the app, and pass the file and network handles to the subprocess. The app can use the ```init_app``` function provided by the platform to initialize the app:

```rust
use dtrust::utils::init_app;

fn main() {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;
}
```

The `init_app` function returns 5 values. 
* `rank` is the index number representing the server that is currently running the app
* `func_name` is the name of the function that the client wants to execute on this app. An app could itself be a single function, or consists of multiple functions. 
* `in_files` and `out_files` are the input files and output files specified by the client. The app is allowed to read from files in `in_files` and write to files in `out_files`. 
* `socks` are sockets that setup pairwise tcp connection between the nodes (servers). 

Notice that every node executes the same piece of code, so we need to use rank to differentiate the nodes and use conditional statements to specify the behaviors of individual node.

Here is an example code to send "Hello world" from node 0 to node 1:
```rust
if rank == 0 {
    socks[1].write("Hello world".as_bytes())?;
} else {
    let mut buffer = [0; 11];
    socks[1].read(&mut buffer)?;
}
```
With network connections all setup, you can now develop your own applications! 



## Example App: Chat
We will now work through an example application developed on top of the dtrust platform...