from collections import defaultdict
import os
import subprocess
import socket
import threading

import dec_exec_pb2 as dec_exec_pb2
import dec_exec_pb2_grpc as dec_exec_pb2_grpc
from directory import AppDirectory, NodeDirectory
import utils

from multiprocessing.connection import Listener

from multiprocessing import reduction

class DecExecServicer(dec_exec_pb2_grpc.DecExecServicer):
    def __init__(self, node_id, config):
        self.node_id = node_id
        self.node_directory = NodeDirectory(config["nodes"])
        self.app_directory = AppDirectory(config["apps"])
        self.storage_dir = os.path.join(config["file_storage_dir"], self.node_id)
        if not os.path.exists(self.storage_dir):
            os.makedirs(self.storage_dir)

        self.node_ids = sorted(self.node_directory.get_all_nodes())
        self.rank = self.node_ids.index(self.node_id)

        self.sock_pool = defaultdict(list)

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

        rank = self.rank
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
        sock_fds = self._setup_pairwise_connections()
        print("Finish setting up socket connections")
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

            thread = threading.Thread(target=self._setup_control_sock, args=(process.pid,))
            thread.start()

            stdout, stderr = process.communicate(
                timeout=10
            )
            print("Printing stdout from the subprocess:")
            print(stdout.decode("utf-8"))
            print("stderr", stderr)
            
        result = dec_exec_pb2.Result(result="success")

        return result 

    def _setup_control_sock(self, child_pid):
        socket_path = f"/tmp/socket-{child_pid}"
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)

        if os.path.exists(socket_path):
            try: 
                os.unlink(socket_path)
            except OSError:
                print(f"Error unlinking Unix domain socket at {socket_path}")
                s.close()
        
        s.bind(socket_path)
        s.listen()
        try:
            conn, _ = s.accept()
            self._command_handler(conn)
        except Exception as e:
            print('server socket connection error: ' + str(e))
            s.close()

    # next step is to send command to the socket
    def _command_handler(self, control_sock):
        while True:
            cmd = utils.recv_msg(control_sock)
            if cmd != None:
                cmd = cmd.decode()
                print("Received command:", cmd)
                cmd_list = cmd.split(" ")
                if cmd_list[0] == "REQUEST_SOCKET":
                    rank1, rank2 = int(cmd_list[1]), int(cmd_list[2])
                    if self.rank in [rank1, rank2]:
                        conn = self._setup_tcp_conn(rank1, rank2)
                        self._pass_socket(control_sock, conn)


    def _setup_tcp_conn(self, rank1, rank2):
        assert(rank1 != rank2)
        if rank1 > rank2:
            rank1, rank2 = rank2, rank1
        id1 = self.node_ids[rank1]
        host1 = self.node_directory.get_node_addr(id1)
        port1 = self.node_directory.get_node_ports(id1)[0]

        if self.rank == rank1:
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind((host1, port1))
            s.listen()
            try:
                conn, addr = s.accept()
                return conn
            except Exception as e:
                print('server socket connection error: ' + str(e))
                s.close()
        elif self.rank == rank2:
            connected = False
            while not connected:
                try:
                    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                    s.connect((host1, port1))
                    connected = True
                    return s
                except Exception as e:
                    pass #Do nothing, just try again  

    def _pass_socket(self, control_sock, handle):
        utils.sendfds(control_sock, [handle.fileno()])

    def _setup_pairwise_connections(self):
        ports = []
        sock_fds = []

        host = self.node_directory.get_node_addr(self.node_id)
        ports = self.node_directory.get_node_ports(self.node_id)

        for i, node in enumerate(self.node_ids):
            if i < self.rank:
                s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                print(f"Listening on {host}:{ports[i]}:")
                s.bind((host, ports[i]))
                s.listen()
                
                clients = []
                try:
                    conn, addr = s.accept()
                    clients.append(conn)
                    data = conn.recv(1024).decode()
                    # print(data)
                    sock_fds.append(os.dup(conn.fileno()))
                    conn.close()
                except Exception as e:
                    print('server socket connection error: ' + str(e))
                    s.close()
            elif i > self.rank:
                host = self.node_directory.get_node_addr(node)
                port = self.node_directory.get_node_ports(node)[self.rank]

                print(f"Connecting to {host}:{port}")
                connected = False
                while not connected:
                    try:
                        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                        s.connect((host,port))
                        connected = True
                    except Exception as e:
                        pass #Do nothing, just try again  

                data = b"hello"
                s.send(data)
                sock_fds.append(os.dup(s.fileno()))
                s.close()
            else:
                s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
                sock_fds.append(s.fileno())

        return sock_fds
