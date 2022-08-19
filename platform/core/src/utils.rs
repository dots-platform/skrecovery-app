use std::io;
use std::fs::File;
#[cfg(unix)]
use std::os::unix::io::FromRawFd;
use std::net::TcpStream;

pub fn init_app() -> io::Result<(u8, String, Vec<File>, Vec<File>, Vec<TcpStream>)> {

    let mut rank = String::new();
    let mut in_fds_str = String::new();
    let mut out_fds_str = String::new();
    let mut sock_fds_str = String::new();
    let mut func_name = String::new();

    io::stdin().read_line(&mut rank)?;
    io::stdin().read_line(&mut in_fds_str)?;
    io::stdin().read_line(&mut out_fds_str)?;
    io::stdin().read_line(&mut sock_fds_str)?;
    io::stdin().read_line(&mut func_name)?;

    let rank: u8 = rank.trim().parse().unwrap();
    let in_fds: Vec<i32> = in_fds_str.split_whitespace().map(|x| x.parse().unwrap()).collect();
    let out_fds: Vec<i32> = out_fds_str.split_whitespace().map(|x| x.parse().unwrap()).collect();
    let sock_fds: Vec<i32> = sock_fds_str.split_whitespace().map(|x| x.parse().unwrap()).collect();

    let mut in_files: Vec<File> = Vec::new();
    let mut out_files: Vec<File> = Vec::new();
    let mut socks: Vec<TcpStream> = Vec::new();

    for fd in in_fds {
        let f = unsafe { File::from_raw_fd(fd) };
        in_files.push(f);
    }
    for fd in out_fds {
        let f = unsafe { File::from_raw_fd(fd) };
        out_files.push(f);
    }
    for fd in sock_fds {
        let s = unsafe { TcpStream::from_raw_fd(fd) };
        socks.push(s);
    }
    
    println!("Finish initializing the app");
    Ok((rank, func_name, in_files, out_files, socks))
}