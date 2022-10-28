import os
import json

class AppDirectory:
    def __init__(self, app_dict):
        self.app_dict = app_dict

    def register_app(self, app):
        pass

    def get_exe_path(self, app_name):
        if app_name not in self.app_dict:
            raise Exception(f"app {app_name} not found")
        return self.app_dict[app_name]["path"]


class NodeDirectory:
    def __init__(self, node_dict):
        self.node_dict = node_dict

    def get_node_addr(self, node_id):
        if node_id not in self.node_dict:
            raise Exception(f"node {node_id} not found")
        node_info = self.node_dict[node_id]
        addr = node_info["addr"].split(":")[0]
        return addr

    def get_node_ports(self, node_id):
        if node_id not in self.node_dict:
            raise Exception(f"node {node_id} not found")
        node_info = self.node_dict[node_id]
        ports = node_info["ports"]
        assert len(ports) == len(self.node_dict) or len(ports) == 1
        if len(ports) == 1:
            p = ports[0]
            ports = [p + i for i in range(len(self.node_dict))]
        return ports 

    def get_all_nodes(self):
        return list(self.node_dict.keys())
        

class ClientDirectory:
    def __init__(self, dir_path):
        with open(dir_path, "r") as f:
            self.client_dict = json.load(f)
    
    def get_client_info(self):
        pass

    def register_client(self):
        pass