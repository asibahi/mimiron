fn main() {
    if let Err(e) = mimiron_lib::run_cli() {
        eprintln!("Encountered error: {e}");
        std::process::exit(1)
    }
}
