use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "disklens", version, about = "High-performance disk space analyzer")]
struct Cli {
    /// Path to analyze (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Maximum scan depth
    #[arg(short = 'd', long)]
    max_depth: Option<usize>,

    /// Maximum concurrent I/O operations
    #[arg(short = 'c', long)]
    concurrency: Option<usize>,

    /// Follow symbolic links
    #[arg(long)]
    follow_symlinks: bool,

    /// Export result as JSON to file (non-interactive mode)
    #[arg(long)]
    export_json: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (logs to stderr)
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Build settings
    let mut settings = disklens::config::settings::Settings::default();
    if let Some(depth) = cli.max_depth {
        settings.max_depth = Some(depth);
    }
    if let Some(conc) = cli.concurrency {
        settings.max_concurrent_io = conc;
    }
    settings.follow_symlinks = cli.follow_symlinks;

    // Resolve path
    let path = std::fs::canonicalize(&cli.path)?;

    // Non-interactive mode: scan and export JSON
    if let Some(ref export_path) = cli.export_json {
        let (event_tx, _rx) = disklens::core::events::create_event_channel();
        let scanner = disklens::core::scanner::Scanner::new(settings, event_tx);
        let result = scanner.scan(path).await?;
        disklens::export::json::export_json(&result, export_path)?;
        println!("Exported to: {}", export_path.display());
        return Ok(());
    }

    // Interactive mode: launch TUI
    let mut app = disklens::app::App::new(path, settings);
    app.run().await
}
