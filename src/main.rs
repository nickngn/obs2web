use clap::Parser;
use obs2web::{build_site, Args};

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    build_site(&args.vault_path, &args.output_dir)?;

    Ok(())
}
