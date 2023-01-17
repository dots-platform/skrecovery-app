# Developing server-side applications

## Setting up servers
We will work through an example of how server-side applications are developed.

`core-modules/template` folder contains a template for server-side applications.

First, we need to set up three servers using the `init_server` command. Currently, the `init_server` command is only compiled for macOS, you can work around it.

On terminal 1:
```
./platform/init_server --node_id node1 --config ./core-modules/template/server_conf_tls.yml
```

On terminal 2:
```
./platform/init_server --node_id node2 --config ./core-modules/template/server_conf_tls.yml
```

On terminal 3:
```
./platform/init_server --node_id node3 --config ./core-modules/template/server_conf_tls.yml
```

Each node will take a global configuration file as input, which contains relevant information about all the nodes. The server configuration file looks like the following:
```yaml
---
use_tls: False                         # set to True if clients and servers should use TLS. See pki/tls-tutorial.md

nodes:
  node1:
    addr: "127.0.0.1:50051"
    ports: [51000, 51001, 51002]      # ports for connection between servers
    # private_key_path: ""tls_certs/node1.test.key"" # Uncomment this field if using TLS
    # pem_cert_path: "tls_certs/node1.test.pem" # Uncomment this field if using TLS
  node2:
    addr: "127.0.0.1:50052"
    ports: [52000]                    # if only one port is specified, the platform will infer the remaining ports. In this case, the ports for node 2 are 52000, 52001, 52002

  node3:
    addr: "127.0.0.1:50053"
    ports: [53000]

apps:
  rust_app:
    path: "./core-modules/template/target/debug/rust_app"

  cpp_app:
    path: "./core-modules/template/a.out"

file_storage_dir: "./core-modules/template/files"
```

In the file, we register three nodes with id `node1, node2, node3`. For each node, we specify the address `addr` that the client connects to and the ports that each node setup pairwise network connections with. Since we have three nodes in the example, each node needs to specify three ports, where port $i$ corresponds to the connection with node $i$. We can also enable TLS for client-server connections by setting `enable_tls` to true and setting `private_key_path` and `pem_cert_path`. See the [TLS tutorial](docs/tls.md) for more details and sample certificates.

The configuration file also registers server-side applications, where each node can call. These are executables that the client can execute on the servers. In our examples, we registered two apps (`rust_app` and `cpp_app`). One written in Rust and another in C++.

Lastly, `file_storage_dir` specifies where the client's uploaded files will be stored.


## Executing server applications
In our current model, all apps are executed by the client. The client-side code is contained in `core-modules/template/client`. Its main function looks like the following:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let node_addrs = ["http://127.0.0.1:50051", "http://127.0.0.1:50052", "http://127.0.0.1:50053"];
    let in_files = [String::from("sk")];

    let cli_id = "user1";
    let app_name = "rust_app";
    let func_name = "";
    let mut client = Client::new(cli_id);

    client.setup(node_addrs.to_vec());
    client.exec(app_name, func_name, in_files.to_vec(), [String::from("out")].to_vec()).await?;
    Ok(())
}
```

In the main function, we specify the servers' addresses, the client's id, and the app name. The client also specifies the input and output files that the app needs access to. When we execute the main function, the client with id `user1` will invoke `rust_app` on the servers. To execute client-side codes, you can create a new terminal, cd into the `core-modules/template` folder, and enters `cargo run --bin client`.

## Writing your own server-side applications


Server-side applications are language-agnostic. You can write them in any language you want, but has to follow the initialization protocol to properly set things up. We provide starter templates for Rust and C++; other languages can implement the initialization protocol similarly.

When the client invokes a particular app, the platform opens the input and output files, setups pairwise tcp connections between the servers, and pass all the file descriptors to the app through stdin. The server app receives all the file descriptors and set them up accordingly. Here is the example starter code for C++, which you can find at `core-modules/template/server/app.cpp`

```c++
int main() {
    string line;
    int rank;
    string func_name;
    std::vector<string> vec;
    std::vector<int> in_fds, out_fds, socks;

    getline(cin, line);
    rank = std::stoi(line);
    getline(cin, line);
    in_fds = split(line, ' ');
    getline(cin, line);
    out_fds = split(line, ' ');
    getline(cin, line);
    socks = split(line, ' ');
    getline(cin, func_name);

    std::vector<FILE*> in_files, out_files;
    FILE *stream;
    for (int i = 0; i < in_fds.size(); ++i) {
        stream = fdopen(in_fds[i], "r");
        in_files.push_back(stream);
    }
    for (int i = 0; i < out_fds.size(); ++i) {
        stream = fdopen(out_fds[i], "r");
        out_files.push_back(stream);
    }

    for (int i = 0; i < in_fds.size(); ++i) {
        std::cout << in_fds[i] << std::endl;
    }

    char* hello = (char *) "Hello world!";
    char buffer[1024] = { 0 };

    if (rank == 0) {
        send(socks[1], hello, strlen(hello), 0);
        send(socks[2], hello, strlen(hello), 0);
    } else if (rank == 1) {
        recv(socks[0], buffer, 1024, 0);
        std::cout << buffer << std::endl;
    } else if (rank == 2) {
        recv(socks[0], buffer, 1024, 0);
        std::cout << buffer << std::endl;
    }

    return 0;
}
```

In the above code snippet, the app receives all the file descriptors from stdin, and sets up files and networks. To test network connections, party 0 sends out the message `Hello world!` to parties 1 and 2, and the other 2 parties receive the message. After everything is set up, you can add your application logic to the app.

## Registering your server-side applications
To register your server-side application, compile the app into executables, and specify the executable's name and file path in the `server_conf_tls.yml` or `server_conf_tcp.yml` file.