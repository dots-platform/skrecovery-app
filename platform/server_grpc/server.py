import os
import subprocess
import socket

import dec_exec_pb2 as dec_exec_pb2
import dec_exec_pb2_grpc as dec_exec_pb2_grpc
from directory import AppDirectory, NodeDirectory

class DecExecServicer(dec_exec_pb2_grpc.DecExecServicer):
    def __init__(self, node_id, config):
        self.node_id = node_id
        self.node_directory = NodeDirectory(config["nodes"])
        self.app_directory = AppDirectory(config["apps"])
        self.storage_dir = os.path.join(config["file_storage_dir"], self.node_id)
        if not os.path.exists(self.storage_dir):
            os.makedirs(self.storage_dir)

        self.node_ids = self.node_directory.get_all_nodes()

    def UploadBlob(self, request, context):
        key = request.key
        val = request.val
        cli_id = request.client_id
        
        dir = os.path.join(self.storage_dir, cli_id)
        if not os.path.exists(dir):
            os.makedirs(dir)
        
        fp = os.path.join(dir, key)
        with open(fp, "wb+") as f:
            f.write(val)
        result = dec_exec_pb2.Result(result="success")
        return result 

    def RetrieveBlob(self, request, context):
        key = request.key
        cli_id = request.client_id
        fp = os.path.join(self.storage_dir, cli_id, key)
        with open(fp, "rb") as f:
            val = f.read()
        result = dec_exec_pb2.Blob(key=key, val=val)
        return result

    def Exec(self, request, context):
        print(f"running app {request.app_name}")

        app_name = request.app_name
        app_uid = request.app_uid
        func_name = request.func_name
        in_files = list(request.in_files)
        out_files = list(request.out_files)
        cli_id = request.client_id
        node_ids = self.node_ids

        rank = node_ids.index(self.node_id)
        exe_path = self.app_directory.get_exe_path(app_name)
        
        # open input files
        in_fds = []
        for fname in in_files:
            fpath = os.path.join(self.storage_dir, cli_id, fname) 
            f = open(fpath, "r")
            fd = os.dup(f.fileno())
            in_fds.append(fd)
            f.close()
        in_fds_str = " ".join(list(map(str, in_fds))) + "\n"

        out_fds = []
        for fname in out_files:
            fpath = os.path.join(self.storage_dir, cli_id, fname)
            f = open(fpath, "w+")
            fd = os.dup(f.fileno())
            out_fds.append(fd)
            f.close()
        out_fds_str = " ".join(list(map(str, out_fds))) + "\n"

        # setting up tcp connections
        sock_fds = self._setup_pairwise_connections(node_ids)
        sock_fds_str = " ".join(list(map(str, sock_fds))) + "\n"

        # execute user-defined functions/apps
        with subprocess.Popen(
            exe_path, stdin=subprocess.PIPE, stdout=subprocess.PIPE, pass_fds=in_fds+out_fds+sock_fds
        ) as process:
            process.stdin.write((str(rank)+"\n").encode())
            process.stdin.write(in_fds_str.encode())
            process.stdin.write(out_fds_str.encode())
            process.stdin.write(sock_fds_str.encode())
            process.stdin.write(func_name.encode())

            stdout, stderr = process.communicate(
                timeout=10
            )
            print("Printing stdout from the subprocess:")
            print(stdout.decode("utf-8"))
            print("stderr", stderr)
            
        result = dec_exec_pb2.Result(result="success")

        return result 
        
    def _setup_pairwise_connections(self, node_ids):
        sock_fds = []

        print("Setting up socket connections")
        for node in node_ids:
            host, port = self.node_directory.get_node_addr(node)

            if node == self.node_id:
                s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                s.bind((host, port))
                s.listen()
                
                clients = []
                try:
                    conn, addr = s.accept()
                    clients.append(conn)
                    data = conn.recv(1024).decode()
                    sock_fds.append(os.dup(conn.fileno()))
                    conn.close()
                except Exception as e:
                    print('server socket connection error: ' + str(e))
                    s.close()
            else:
                s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                s.connect((host, port))        
                data = b"hello"
                s.send(data)
                sock_fds.append(os.dup(s.fileno()))
                s.close()
        
        print("Finish setting up socket connections")
        return sock_fds