use std::io;
use std::io::prelude::*;
use dtrust::utils::init_app;

fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {:?}", rank);
    println!("func name {:?}", func_name);

    // testing network connections
    if rank == 0 {
        socks[0].write("Hello world".as_bytes())?;
        let mut buffer = [0; 30];
        socks[1].read(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
    } else {
        let mut buffer = [0; 11];
        socks[0].read(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
        socks[1].write("Hello from the other side".as_bytes())?;
    }

    // printing input files
    for mut f in in_files {
        let mut buf = [0; 1024];
        f.read(&mut buf)?;
        println!("file content: {}", String::from_utf8_lossy(&buf));
    }
    Ok(())

    // develope server side application here ...
}