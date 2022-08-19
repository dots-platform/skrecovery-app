
### Starting servers
```bash
./platform/init_server --node_id node1 --config ./app/server_conf.yml

./platform/init_server --node_id node2 --config ./app/server_conf.yml
```

### Running the client
```bash
cd app
cargo build
cargo run --bin client
```