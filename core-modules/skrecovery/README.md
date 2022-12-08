# Secret Key Recovery 
**Group Members**: Allison Li, Michael Ren, Yuwen Zhang \
**Google slide presentation**: [Link to slides](https://docs.google.com/presentation/d/1u1Br2Mme98Wht2vrZYd0OYpvr5GDZNZZfiXFLk-RHWU/edit?usp=sharing)

## What it is 
The SK recovery module provides a framework for a secret key recovery protocol distributed across multiple nodes. \
Users can upload their secret key along with a password which can be used to recover the secret key. These pieces of data are sharded on the client side before being uploaded to any servers.\
 When a user wants to recover their key, they can submit a password guess. Each server gets a shard of the guess, and then they perform an MPC to verify if the password is correct. If so, they send their shards of the secret key back to the user. 

## Design and Architecture 
TODO 

## Threat Model 
Under the assumption that Alice shards her secret key into `N` shares: 
* Servers are semi-honest: compromised servers will still faithfully execute the protocol.
* There are no more than `N-1` colluding MPC servers. 
* DOS attacks are not possible. 

If these requirements are met, then it is guaranteed that Alice can recover her secret key, and an attacker who doesn't know Alice's password can recover Alice's secret key with negligible probability. 


# Running the program 

### 0. Install Dependencies 
* Install [rustup](https://rustup.rs/) 
* Install Rust using rustup: `rustup install 1.65.0`
* Install [yq](https://github.com/mikefarah/yq)

### 1. Start the nodes 
Run the following commands in a single terminal.
```jsx
./platform/init_server --node_id node1 --config ./core-modules/skrecovery/server_conf.yml
```

```jsx
./platform/init_server --node_id node2 --config ./core-modules/skrecovery/server_conf.yml
```

(MAYBE NOT THIS ONE)
```jsx
./platform/init_server --node_id node3 --config ./core-modules/skrecovery/server_conf.yml
```

### 2. Start the server
Start the server in another terminal. 
```jsx
cd core-modules/skrecovery
cargo build
cargo run --bin rust_app
```

### 3. Commands 
**Upload a secret key and password**

```jsx
$ cargo run --bin client upload_sk_and_pwd my_id my_sk my_pwd
```
**Recover the secret key with a password guess** 
```jsx
$ cargo run --bin client recover_sk my_id my_pwd
```

# Dependencies 
See `Cargo.toml` for dependencies and `Cargo.lock` for the specific versions. 
The following other dependencies should be preinstalled on the system as well: 
* rustc 1.65.0 (897e37553 2022-11-02) 
* [yq](https://github.com/mikefarah/yq/) 4.30.5 

# Tests 
We tested the server by running the code ourselves and feeding it correct and incorrect password guesses. 

## Terminal output from a correct guess 
```bash
TODO 
```

## Terminal output from an incorrect guess 
```bash
TODO 
```
