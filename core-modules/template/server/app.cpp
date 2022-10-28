#include <iostream>
#include <string>
#include <sstream>
#include <vector>
#include <iterator>
#include <stdio.h>

#include <unistd.h>
#include <sys/types.h> 
#include <sys/socket.h>
#include <netinet/in.h>

using namespace std;

template <typename Out>
void split(const std::string &s, char delim, Out result) {
    std::istringstream iss(s);
    std::string item;
    while (std::getline(iss, item, delim)) {
        *result++ = std::stoi(item);
    }
}

std::vector<int> split(const std::string &s, char delim) {
    std::vector<int> elems;
    split(s, delim, std::back_inserter(elems));
    return elems;
}

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