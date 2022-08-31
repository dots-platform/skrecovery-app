# DTrust: The Berkeley Decentralized Trust Stack

***Caveat: This codebase is currently close-sourced. We plan to have it open-sourced at the end of the class. Until then, please do not distribute this repo to anyone outside this class.***

DTrust is a platform for developing applications with distributed trust. 

## Getting Started
Our platform can run on MacOS and Linux. Windows is currently not supported. 

### 1. Installing Rust
Follow the instruction on [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install) to install Rust on your machine.

### 2. Initializing decentralized nodes
First, we need to initialize the DTrust platform on multiple servers. We can use the `init_server` command to initialize a private node on a server. The `init_server` command takes a `node_id` and config file (`server_conf.yml`) as input, and initialize the node according to the config. The bash script below will initialize two servers with `node1` and `node2` as their `node_id` respectively.  You need to open two terminals to execute the following commands.

On terminal 1:
```bash
./platform/init_server --node_id node1 --config ./core-apps/pki/server_conf.yml
```

On terminal 2:
```bash
./platform/init_server --node_id node2 --config ./core-apps/pki/server_conf.yml
```

### 3. Running an example application
The `core-apps/pki` folder contains an example application called distributed PKI (public key infrastructure) written in Rust. This app enables a client to store his public key on multiple nodes. Other clients who want to talk to this client can then retrieve the public key from these servers. To run the client, open a another terminal and type in the following commands. 

```bash
cd core-apps/pki
cargo build
cargo run --bin client "upload_pk" "user1" "random_public_key"
cargo run --bin client "recover_pk" "user1"
```

You should see the message: "recovered public-key: "random_public_key"" at the bottom of your screen.

To learn more about the DTrust platform as well as this example application, checkout our [tutorial](tutorial.md). The tutorial will equip you with the necessary knowledge to develop your own decentralized applications on DTrust.
