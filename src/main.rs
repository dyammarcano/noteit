fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match noteit::cli::run(&args) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("noteit: {e}");
            std::process::exit(1);
        }
    }
}
