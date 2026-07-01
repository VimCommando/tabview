fn main() -> anyhow::Result<()> {
    let args = tabview::cli::Args::parse_args();
    tabview::run(args)
}
