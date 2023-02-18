# Instructions to execute the app

### Compile the C++ app
Place yourself under the `dtrust/core-modules/template` folder
```bash
g++ ./cpp_app/main.cpp -I ./cpp_app
```

### Start the app
Place yourself under the `dtrust` folder.
```bash
python3 ./platform/server_grpc/init_server.py  --node_id ${node_id} --config ./core-modules/template/server_conf_tcp.yml
```

### Invoke the C++ app
``` bash
cargo run --bin client
```