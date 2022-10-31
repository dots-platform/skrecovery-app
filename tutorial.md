# Developing your decentralized application

### Developing Client-side Computations (Step 1, Step 3)
The BDots platform provides the `upload_blob` and `retrive_blob` functions to upload and retrieve any data as files.

Before uploading or downloading any data, the client first needs to specify which nodes it will connect to:
```rust
let cli_id = "user1";
let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052"];
let mut client = Client::new(cli_id);
client.setup(node_addrs.to_vec());
```

Then, the client can upload data and retrieve data from the servers using the provided functions. The `upload_blob` function takes a string as key, and a list of bytes as values. It will upload the i'th bytes in the list to the i'th server, and store it in a file indexed by the key. The `retrieve_blob` function takes a key string as input, and retrieve the previously uploaded files indexed by the key. Here is an example to upload the strings "hello" and "world" to the servers.  

```rust
let vals = vec!["hello".as_bytes().to_vec(), "world".as_bytes().to_vec()];
client.upload_blob(String::from("blob_name"), vals);
let blob = client.retrieve_blob(String::from("blob_name"));
```

<!-- Here is an example client function that uploads a public key to all servers using `upload_blob` and recover them through `retrieve_blob`.
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
``` -->

### Developing a decentralized app on the servers (Step 2)
A decentralized app on the server is an executable that the servers jointly executes. The client invokes the executable through gRPC requests, and specify the names of the input files and output files. The BDots platform first open these files and setups network connections. Then, the BDots platform forks a subprocess to execute the app, and pass the file and network handles to the subprocess. The app can use the ```init_app``` function provided by the platform to initialize the app:

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
With pairwise network connections all setup, you can now develop your own applications! You can use `core-modules/template` as the starter code for your application. 



## Example App: Chat
We will now work through an example application developed on top of the BDots platform.

We use a [python-chat application](https://github.com/dtrust-project/dtrust-applications/tree/master/python-chat-dtrust) as our example here.

In this application, clients use Elliptic-curve Diffie Hellman key exchange to set up symmetric keys which are used to encrypt all conversation that happens between the clients. This provides end-to-end encryption as the central coordinating server doesn't get to see the messages in the conversations in clear. 

Our starting point is the [python-chat application by Ludvig Knutsmark](https://github.com/ludvigknutsmark/python-chat). This application provides end-to-end encryption, but clients have to trust the central coordinating server to convey their public keys to other clients. This server can become a central point-of-attack, and if compromised, can start sending incorrect public keys leading to a man-in-the-middle attack which completely undermines end-to-end encryption.

To fix this issue, we use our distributed trust platform to get rid of the central point-of-attack by using a distributed public-key infrastructure (PKI) which maintains public keys of all clients. We first look into how the distributed PKI is built using our platform, and later look into how the python-chat application uses it to provide stronger end-to-end encryption guarantees.

### Distributed PKI
The idea behind our distributed PKI is to have more than one decentralized node maintain a copy of clients' public key (clients who want to chat with each other), and whenever a client wants to talk to another client, they can simply retrieve the copies from all decentralized nodes, compare the copies for equality, and if they all match, the client can be quite certain than they won't suffer a man-in-the-middle attack; this is because the likelihood that all the decentralized nodes are acting malicious / compromised simultaneously is very low.

Here is a code walkthrough:

Clients use this function to upload their public to the distributed PKI by sending a copy to each decentralized node
```rust
async fn upload_pk(&self, id: String, key: String) {
        let upload_val = vec![key.as_bytes().to_vec(); self.node_addrs.len()];
        self.upload_blob(id, upload_val).await;
}
```
and this function to retrieve the copies from the decentralized nodes and compare them for equality before accepting them for use in the chat application
```rust
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
        println!("Recovered public-key {:?}", key);
        key
}
```
The `main` function in `core-modules/pki/client/main.rs` takes as argument which function to call, i.e., `upload_pk` or `recover_pk`, and can be called directly from external chat application as we see next.

### Target Chat Application
Applications are all hosted under the [dtrust-applications repo](https://github.com/dtrust-project/dtrust-applications). We focus on the [python-chat application](https://github.com/dtrust-project/dtrust-applications/tree/master/python-chat-dtrust) here.

To add the support for distributed PKI in this application, we direct our focus on its Diffie-Hellman key exchange (DHKE) code in `Client/dh.py`. There are 2 rounds in a DHKE. In the first round, the non-hub client (for definition of hub/non-hub, refer to python-chat documentation) sends its public key to the hub client via the central coordinating server, and in the second round, the hub client completes DHKE and the shared symmetric key is established between the two clients. This interaction is prone to man-in-the-middle attack by a compromised central coordinating server who can plug in its own public key rather than using the client's key. In this tutorial, we safeguard the first round of this interaction using a distributed PKI; we don't safeguard the second round and leave that as an exercise.

In the first round, rather than the hub client getting the public key of non-hub client through the central server, it uses our distributed PKI to fetch the public key. The workflow is as follows:

The non-hub client creates its public key and calls into `core-modules/pki/client/main.rs` of our framework with `upload_pk` as the argument
```python
def keyExchange(self_alias):
    serialized_public = clientCreateKeys()
    pubkey = serialized_public
    print("Distributed Trust Stack Call: Uploading pk...")
    os.system('cd ../../../dtrust/core-modules/pki; cargo run --bin client upload_pk ' + str(self_alias) + ' "' + str(pubkey) + '"')
```

Whenever the hub client wants to do DHKE with non-hub client, it can retrieve the latter's public key by calling into `core-modules/pki/client/main.rs` with `recover_pk`
```python
def sendFernet(s, data, fernet_key, peer_alias):
    print("Distributed Trust Stack Call: Retrieving pk...")
    pubkey = subprocess.check_output(['cd ../../../dtrust/core-modules/pki; cargo run --bin client recover_pk ' + str(peer_alias)], shell=True)
    # Some formatting of the received key
    pubkey = pubkey.splitlines()[-1][1:-1].split(r'\n')[:-1]
    pub = pubkey[0] + "\n" + pubkey[1] + "\n" + pubkey[2] + "\n" + pubkey[3] + "\n" + pubkey[4] 
    pub = pub.encode("utf8")
    client_public = serialization.load_pem_public_key(
        pub,
        backend=default_backend()
    )
    ...
```