#![deny(warnings)]
#![deny(clippy::pedantic)]
#![warn(rust_2018_idioms)]

#[tokio::main]
async fn main() {
    let path = std::env::args()
        .nth(1)
        .map_or_else(|| {
            eprintln!("No path provided");
            std::process::exit(-1);
        }, |path_string| {
            let path = std::path::PathBuf::from(path_string.clone());

            if !path.exists() {
                eprintln!("Path provided does not exist: {}", &path_string);
                std::process::exit(-1);
            }

            if !path.is_dir() {
                eprintln!("Path provided is not a directory: {}", &path_string);
                std::process::exit(-1);
            }

            path
        });

    let port = std::env::args()
        .nth(2)
        .map_or(Ok(3030_u16), |arg| arg.parse::<u16>())
        .unwrap_or_else(|err| {
            eprintln!("Invalid port: {}", err);
            std::process::exit(-1);
        });

    println!("Launching '{}' on http://0.0.0.0:{}", path.display(), port);

    use warp::Filter;
    warp::serve(warp::any().and(warp::fs::dir(path))).run(([0, 0, 0, 0], port)).await;
}
