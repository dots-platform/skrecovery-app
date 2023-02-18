#include <iostream>
#include <string>
#include <sstream>
#include <vector>
#include <iterator>
#include <stdio.h>

#include <unistd.h>
#include <sys/types.h> 
#include <sys/un.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

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

#define CMSG_SIZE CMSG_SPACE(sizeof(int))
/* 
 * _sendfd(): send a message and piggyback a file descriptor.
 *
 * Note that the file descriptor cannot be sent by itself, at least one byte of
 * payload needs to be sent.
 *
 * Parameters:
 *  sock: AF_UNIX socket
 *  fd:   file descriptor to pass
 *  len:  length of the message
 *  msg:  the message itself
 *
 * Return value:
 *  On success, sendfd returns the number of characters from the message sent,
 *  the file descriptor information is not taken into account. If there was no
 *  message to send, 0 is returned. On error, -1 is returned, and errno is set
 *  appropriately.
 *
 */
int _sendfd(int sock, int fd, size_t len, const void *msg) {
    struct iovec iov[1];
    struct msghdr msgh;
    char buf[CMSG_SIZE];
    struct cmsghdr *h;
    int ret;

    /* At least one byte needs to be sent, for some reason (?) */
    if(len < 1)
        return 0;

    memset(&iov[0], 0, sizeof(struct iovec));
    memset(&msgh, 0, sizeof(struct msghdr));
    memset(buf, 0, CMSG_SIZE);

    msgh.msg_name       = NULL;
    msgh.msg_namelen    = 0;

    msgh.msg_iov        = iov;
    msgh.msg_iovlen     = 1;

    msgh.msg_control    = buf;
    msgh.msg_controllen = CMSG_SIZE;
    msgh.msg_flags      = 0;

    /* Message to be sent */
    iov[0].iov_base = (void *)msg;
    iov[0].iov_len  = len;

    /* Control data */
    h = CMSG_FIRSTHDR(&msgh);
    h->cmsg_len   = CMSG_LEN(sizeof(int));
    h->cmsg_level = SOL_SOCKET;
    h->cmsg_type  = SCM_RIGHTS;
    ((int *)CMSG_DATA(h))[0] = fd;

    ret = sendmsg(sock, &msgh, 0);
    return ret;
}
/* 
 * _recvfd(): receive a message and a file descriptor.
 *
 * Parameters:
 *  sock: AF_UNIX socket
 *  len:  pointer to the length of the message buffer, modified on return
 *  buf:  buffer to contain the received buffer
 *
 * If len is 0 or buf is NULL, the received message is stored in a temporary
 * buffer and discarded later.
 *
 * Return value:
 *  On success, recvfd returns the received file descriptor, and len points to
 *  the size of the received message. 
 *  If recvmsg fails, -1 is returned, and errno is set appropriately.
 *  If the received data does not carry exactly one file descriptor, -2 is
 *  returned. If the received file descriptor is not valid, -3 is returned.
 *
 */
int _recvfd(int sock, size_t *len, void *buf) {
    struct iovec iov[1];
    struct msghdr msgh;
    char cmsgbuf[CMSG_SIZE];
    char extrabuf[4096];
    struct cmsghdr *h;
    int st, fd;

    if(*len < 1 || buf == NULL) {
        /* For some reason, again, one byte needs to be received. (it would not
         * block?) */
        iov[0].iov_base = extrabuf;
        iov[0].iov_len  = sizeof(extrabuf);
    } else {
        iov[0].iov_base = buf;
        iov[0].iov_len  = *len;
    }
    
    msgh.msg_name       = NULL;
    msgh.msg_namelen    = 0;

    msgh.msg_iov        = iov;
    msgh.msg_iovlen     = 1;

    msgh.msg_control    = cmsgbuf;
    msgh.msg_controllen = CMSG_SIZE;
    msgh.msg_flags      = 0;
        
    st = recvmsg(sock, &msgh, 0);
    if(st < 0)
        return -1;

    *len = st;
    h = CMSG_FIRSTHDR(&msgh);
    /* Check if we received what we expected */
    // std::cout << "h->cmsg_len: " << h->cmsg_len << std::endl;
    // std::cout << "h->msg_level: " << h->cmsg_level << std::endl;
    // std::cout << "h->msg_type: " << h->cmsg_type << std::endl;

    if(h == NULL
            || h->cmsg_len    != CMSG_LEN(sizeof(int))
            || h->cmsg_level  != SOL_SOCKET
            || h->cmsg_type   != SCM_RIGHTS) {
        return -2;
    }
    fd = ((int *)CMSG_DATA(h))[0];
    if(fd < 0)
        return -3;
    return fd;
}

int get_control_sock(const char* socket_path) {

    // create socket
    int socket_fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (socket_fd == -1) {
        std::cerr << "Error creating socket\n";
        return 1;
    }

    // set socket address
    struct sockaddr_un socket_addr;
    socket_addr.sun_family = AF_UNIX;
    strncpy(socket_addr.sun_path, socket_path, sizeof(socket_addr.sun_path) - 1);

    // connect to server
    if (connect(socket_fd, (struct sockaddr*)&socket_addr, sizeof(socket_addr)) == -1) {
        std::cerr << "Error connecting to socket\n";
        return 1;
    }

    return socket_fd;
}

int sendmsg(int sock_id, const char* msg) {
    // Get the length of the message
    size_t msg_len = strlen(msg);

    // Create a buffer for the message and length
    size_t buf_len = msg_len + sizeof(uint32_t);
    char* buf = new char[buf_len];

    // Append the length of the message as a 4-byte integer to the front of the buffer
    uint32_t len = htonl(msg_len);
    memcpy(buf, &len, sizeof(len));

    // Copy the message to the rest of the buffer
    memcpy(buf + sizeof(len), msg, msg_len);

    // Send the buffer through the socket
    ssize_t bytes_sent = send(sock_id, buf, buf_len, 0);

    // Check for errors
    if (bytes_sent == -1) {
        // Print an error message
        fprintf(stderr, "Error sending message: %s\n", strerror(errno));
        return -1;
    } else if (bytes_sent < buf_len) {
        // Print a warning message if only part of the message was sent
        fprintf(stderr, "Warning: Only sent %ld of %ld bytes\n", bytes_sent, buf_len);
    }

    // Free the buffer
    delete[] buf;

    return 0;
}

class DoTSServer {
public:
    int rank;
    int control_sock; // socket to the parent process
    std::vector<int> in_fds;  // input file descriptors
    std::vector<int> out_fds; // output file descriptors
    std::vector<int> socks;  // pairwise sockets between parties
    const char* socket_path; // path for control socket

    DoTSServer() {
        _init_dots_app();
    }

    int request_sock(int rank1, int rank2) {
        std::string cmd = "REQUEST_SOCKET " + std::to_string(rank1) + " " + std::to_string(rank2);
        sendmsg(control_sock, cmd.c_str());

        char buf[1024];
        size_t len = 1024;
        if (this->rank == rank1 || this->rank == rank2) {
            int conn = _recvfd(control_sock, &len, buf);
            return conn;
        }
        return -1;
    }

private:
    void _init_dots_app() {
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

        
        pid_t pid = getpid();
        std::string path = "/tmp/socket-" + std::to_string(pid);
        const char *socket_path = path.c_str();
        int control_sock = get_control_sock(socket_path);

        this->rank = rank;
        this->in_fds = in_fds;
        this->out_fds = out_fds;
        this->socks = socks;
        this->control_sock = control_sock;
        this->socket_path = socket_path;
    }
};