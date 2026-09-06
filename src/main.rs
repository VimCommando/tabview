#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let args = tview::cli::Args::parse_args();
    tview::run(args)
}
