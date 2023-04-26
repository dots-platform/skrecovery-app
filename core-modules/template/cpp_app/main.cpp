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
#include <arpa/inet.h>

#include "init_dots.h"

using namespace std;

int main() {
    DoTSServer dots;
    int sock = dots.request_sock(0, 1);

    if (dots.rank == 0) {
        char* hello = (char *) "Hello world!";
        // send_msg(sock, hello);
        send(sock, hello, strlen(hello), 0);
    } else if (dots.rank == 1) {
        char buffer[1024] = { 0 };
        recv(sock, buffer, 1024, 0);
        std::cout << "received msg: " << buffer << std::endl;
    }

    return 0;
}