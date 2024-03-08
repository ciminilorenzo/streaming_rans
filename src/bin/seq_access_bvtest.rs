use clap::Parser;
use dsi_progress_logger::{ProgressLog, ProgressLogger};
use folded_streaming_rans::bvgraph::random_access::ANSBVGraph;
use lender::Lender;
use webgraph::prelude::SequentialLabeling;

#[derive(Parser, Debug)]
#[command(about = "Tests the speed of an ANS graph", long_about = None)]
struct Args {
    /// The basename of the graph.
    basename: String,
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    stderrlog::new()
        .verbosity(2)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();

    let mut pl = ProgressLogger::default();
    let graph = ANSBVGraph::load(args.basename)?;

    pl.item_name("node")
        .expected_updates(Some(graph.num_nodes() * 10));
    pl.start("Starting sequential-access speed test...");
    let mut c: u64 = 0;

    for _ in 0..10 {
        let start = std::time::Instant::now();
        let mut iter = graph.iter();
        while let Some((_, succ)) = iter.next() {
            c += succ.into_iter().count() as u64;
            pl.update();
        }
        println!("{}", (start.elapsed().as_secs_f64() / c as f64) * 1e9);
    }
    pl.done_with_count(c as usize);
    Ok(())
}