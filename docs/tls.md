# Using TLS in DoTS Example

To indicate servers should only accept TLS connections, the `use_tls` parameter must be specified in the server configuration YAML file (`use_tls` can be set to false to initiate TCP connections). Clients, when initializing the `Client` object, provide the filepath of the root CA certificate file to indicate which certificates ought to be trusted.

We use the [Tonic](https://github.com/hyperium/tonic) library to establish connections between clients and servers, as this library was the existing connection library of choice in DTrust. 

## Background: Enabling TCP in the Example App
To use TCP communication between clients and servers, set `let use_tls: bool = false` in main.rs of the client application.

Then, start the servers with the updated configurations:
```
python3 ./platform/server_grpc/init_server.py --node_id node1 --config ./core-modules/pki/server_conf_tcp.yml
python3 ./platform/server_grpc/init_server.py --node_id node2 --config ./core-modules/pki/server_conf_tcp.yml
```

## Setup
When using TCP, the test PKI application uses the computer's localhost to create 2 separate servers at different ports. TLS, in contrast, must connect to domain names because TLS certificates cannot be issued for IP addresses. The Server config files use dummy names node1.test and node2.test so self-signed certificates can be issued for both servers. In order to map these domains to localhost (which is necessary to simulate networking communication), add the following lines to your `/etc/hosts` file:

```
127.0.0.1               node1.test
127.0.0.1               node2.test
```

## Enabling TLS in the Example App
To enable TLS between clients and servers, set `let use_tls: bool = true` in main.rs of the client application.

Then, start the servers with the updated configurations:
```
python3 ./platform/server_grpc/init_server.py --node_id node1 --config ./core-modules/pki/server_conf_tls.yml
python3 ./platform/server_grpc/init_server.py --node_id node2 --config ./core-modules/pki/server_conf_tls.yml
```

## Adding TLS support to your own App
### Real-life deployment
A limitation of the Rust TLS library is that RusTLS can only authenticate on domain names, not IP addresses. For this reason, servers will need domain names. For TLS authentication to work, servers will need to request a TLS certificate from an existing certificate authority (ex: Lets Encrypt). Include a path to your DTrust Server's certificate in the server config YAML file.

### Test (Local) deployment
Local testing requires the simulation of multiple machines on a single, local, device. As mentioned earlier, RusTLS is constrained to only perform TLS authentication over domain names. It is thus necessary to emulate individual domain names for servers by modifying your `etc/hosts` file as modeled in the "Setup" section of this tutorial; specifically, a test domain name ought to be created for each DTrust server (running locally) and mapped to your machine's localhost.

Once the test domain names are configured, a certificate must be created for each of them. Local testing necessitates the creation of a self-signed test CA, tutorials for which can be found online. Once said CA has been created, it can be used to issue TLS certificates for each DTrust server running locally. The self-signed CA's certificate path is configured in the `main.rs` file of the example application, and the server certificates are similarly configured in the server config YAML. Examples can be found in this application's `tls_certs/` folder. `myCA.key` and `myCA.pem` are the private key and public certificate (respectively) of the self-signed root CA. `node{1,2}.com.key` and `node{1,2}.com.cert` are the private key and public certificate of server nodes 1 and 2 (respectively).

If you need to generate any additional certificates for `nodex`:
```
openssl req -newkey rsa:2048 -nodes -keyout nodex.test.key -out node3.test.csr -subj "/CN=nodex.test"
openssl x509 -req -in nodex.test.csr -CA myCA.pem -CAkey myCA.key -CAcreateserial -out node3.test.crt -days 365 -extensions v3_req -extfile v3.cnf
```

where `v3.cnf` is

```
[ v3_req ]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
subjectAltName = DNS:nodex.test
```

### Server Configuration
The field `use_tls` in the server config file should be set to `true`, and public keys and private keys should be specified for each nodes as filepaths in the `pem_cert_path` and `private_key_path` yaml fields respectively.

See `dtrust/tutorial2.md` for a more detailed description for initializing servers; the configuration files used for the PKI application above may also serve as useful examples.

### Client Configuration
When initializing the `Client` object, clients intending to use TLS must supply `https` addresses to nodes and the path to the system root certificate for authenticating certificates. On mac, the only platform currently supported by DTrust, this file is found at `/etc/ssl/cert.pem`. For testing, you can use the self-signed CA found under `tls_certs/myCA.{key,pem}` (the password for the private key is `dtrustdemo`).