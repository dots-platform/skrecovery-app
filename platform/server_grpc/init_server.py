import argparse
from concurrent import futures

import grpc
import dec_exec_pb2_grpc as dec_exec_pb2_grpc
from server import DecExecServicer

import yaml

def serve():
    parser = argparse.ArgumentParser(description="")
    parser.add_argument('--node_id', type=str, required=True)
    parser.add_argument('--config', type=str, required=True)

    args = parser.parse_args()
    with open(args.config, "r") as f:
        config = yaml.safe_load(f)
    
    node_id = args.node_id
    if node_id not in config["nodes"]:
        raise Exception(f"node {node_id} not found in config")
    addr = config["nodes"][node_id]["addr"]

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    dec_exec_pb2_grpc.add_DecExecServicer_to_server(
    DecExecServicer(node_id, config), server
    )
    server.add_insecure_port(f'{addr}')
    print(f"Starting server {node_id} on {addr}")
    server.start()
    server.wait_for_termination()


if __name__ == "__main__":
    serve()