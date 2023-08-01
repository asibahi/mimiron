fn main() {
    if let Err(e) = mimiron::run() {
        println!("Encountered error: {e}");
        std::process::exit(1)
    }
}
