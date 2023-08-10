fn main() {
    match mimiron::run() {
        Err(e) => {
            eprintln!("Encountered error: {e}");
            std::process::exit(1)
        }
        Ok(msg) => println!("{msg}"),
    }
}
