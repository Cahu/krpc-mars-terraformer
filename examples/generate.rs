use krpc_mars_terraformer as teraform;

use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    file: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    teraform::run(std::iter::once(opts.file), "/tmp/")?;
    Ok(())
}
