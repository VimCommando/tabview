#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let args = tabview::cli::Args::parse_args();
    tabview::run(args)
}
