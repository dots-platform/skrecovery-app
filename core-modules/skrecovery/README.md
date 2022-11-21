# starting this thing up is a big pain

1. Start some nodes

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


2. Start the server

```jsx
cd core-modules/skrecovery
cargo build
cargo run --bin rust_app
```

3. 

upload some sk's and pwds

```jsx
cargo run --bin client upload_sk_and_pwd 
```


