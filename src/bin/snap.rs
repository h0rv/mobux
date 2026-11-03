use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "shots".to_string());
    match mobux::shot::write_home(Path::new(&dir)) {
        Ok(paths) => {
            for path in paths {
                println!("{}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}
