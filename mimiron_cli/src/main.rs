fn main() {
    if let Err(e) = mimiron::run_cli() {
        eprintln!("Encountered error: {e}");
        std::process::exit(1)
    }
}
