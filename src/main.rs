fn main() {
    if let Err(e) = mimiron::run() {
        eprintln!("Encountered error: {e}");
        std::process::exit(1)
    }
}
