use std::io;
use std::io::prelude::*;
use dtrust::utils::init_app;

fn main() -> io::Result<()> {
    let (rank, func_name, in_files, out_files, mut socks) = init_app()?;

    println!("rank {:?}", rank);
    println!("func name {:?}", func_name);
    println!("socks0: {:?}", socks[0]);
    println!("socks1: {:?}", socks[1]);
    println!("socks2: {:?}", socks[2]);

    // testing network connections
    if rank == 0 {
        socks[1].write("Hello world".as_bytes())?;
        let mut buffer = [0; 30];
        socks[1].read(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
        socks[2].read(&mut buffer)?;
        
        println!("READ FRIN SOCKS 2 ∫{}", String::from_utf8_lossy(&buffer));
    } else if rank == 1 {
        let mut buffer = [0; 11];
        socks[0].read(&mut buffer)?;
        println!("{}", String::from_utf8_lossy(&buffer));
        socks[0].write("Hello from party 1".as_bytes())?;
    } else {
        socks[0].write("Hello from party 2".as_bytes())?;
    }

    // printing input files
    for mut f in in_files {
        let mut buf = [0; 1024];
        f.read(&mut buf)?;
        println!("file content: {}", String::from_utf8_lossy(&buf));
    }
    Ok(())
    // develop server side application here ...
}