use std::io;
use dtrust::utils::init_app;

fn main() -> io::Result<()> {
    // underscores are added to remove comments
    let (_rank, _func_name, _in_files, _out_files, mut _socks) = init_app()?;
    Ok(())
}