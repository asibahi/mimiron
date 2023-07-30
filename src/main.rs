use clap::Parser;

fn main() {
    let args = mimiron::MimironArgs::parse();
    if let Err(e) = mimiron::run(args) {
        println!("Encountered error: {e}");
        std::process::exit(1)
    }
}
