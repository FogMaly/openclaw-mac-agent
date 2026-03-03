mod client;
mod config;
mod executor;
mod protocol;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cfg = match config::Config::load() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("config error: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = client::run_forever(cfg).await {
        eprintln!("fatal error: {e}");
        std::process::exit(2);
    }
}
