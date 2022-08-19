# DTrust: The Berkeley Decentralized Trust Stack

DTrust is a platform for developing applications with distributed trust.

## Getting Started

### Initializing decentralized nodes
First, we need to initialize the DTrust platform on multiple servers. We can use the `init_server` command to initialize a private node on a server. The `init_server` command takes a `node_id` and config file (`server_conf.yml`) as input, and initialize the node according to the config. The script below will initialize two servers with `node1` and `node2` as their `node_id` respectively. 

```bash
./platform/init_server --node_id node1 --config ./app/server_conf.yml

./platform/init_server --node_id node2 --config ./app/server_conf.yml
```

### Running an example application
The `app` folder provides an example application written in Rust. This application enables a user to distribute her secret keys to multiple nodes as secret shares for secure storage, and retrieve them when needed. 

```bash
cd app
cargo build
cargo run --bin client
```
This script will execute the main function in `app/client/main.rs`. In this example, the client distribute her key to the nodes by calling `distribute_sk()`, executes the server-side example app (in `app/server/main.rs`) on these nodes, and retrieve back her key by calling `retrieve_key()`