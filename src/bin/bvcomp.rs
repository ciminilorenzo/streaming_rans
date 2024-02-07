use anyhow::Result;
use clap::Parser;
use dsi_bitstream::prelude::*;
use dsi_progress_logger::*;
use epserde::prelude::Serialize;
use folded_streaming_rans::bvgraph::mock_writers::{EntropyEstimator, Log2Estimator};
use folded_streaming_rans::bvgraph::writer::{BVGraphMeasurableEncoder, BVGraphModelBuilder};
use lender::*;
use mem_dbg::{DbgFlags, MemDbg};
use std::path::PathBuf;
use webgraph::prelude::*;

#[derive(Parser, Debug)]
#[command(about = "Recompress a BVGraph", long_about = None)]
struct Args {
    /// The basename of the graph.
    basename: String,

    /// The basename for the newly compressed graph.
    new_basename: String,

    #[clap(flatten)]
    ca: CompressArgs,
}

pub fn main() -> Result<()> {
    let args = Args::parse();
    let mut pl = ProgressLogger::default();
    let seq_graph = BVGraph::with_basename(&args.basename)
        .endianness::<BE>()
        .load()?;

    stderrlog::new()
        .verbosity(2)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();

    // create a log2 mock writer, where the cost of each symbol is the amount of bits needed to represent it
    let log2_mock = Log2Estimator::build();
    // create a builder that uses the log2 mock writer
    let model_builder = BVGraphModelBuilder::<Log2Estimator>::new(log2_mock);
    let mut bvcomp = BVComp::<BVGraphModelBuilder<Log2Estimator>>::new(
        model_builder,
        args.ca.compression_window,
        args.ca.min_interval_length,
        args.ca.max_ref_count,
        0,
    );

    pl.item_name("node")
        .expected_updates(Some(seq_graph.num_nodes()));
    pl.start("Pushing input into the model builder with log2 mock...");

    // first iteration: build a model with the log2 mock writer
    for_![ (_, succ) in seq_graph {
        bvcomp.push(succ)?;
        pl.update();
    }];
    pl.done();

    pl.start("Building the model with log2 mock...");
    // get the ANSModel4Encoder obtained from the first iteration
    let model4encoder = bvcomp.flush()?.build();
    pl.done();
    // get the folding parameters from the model, that is the best combination of radix and fidelity
    let folding_params = model4encoder.get_folding_params();
    // create a new table of costs based on params obtained from the previous step
    let entropy_estimator = EntropyEstimator::new(&model4encoder, folding_params);
    let model_builder = BVGraphModelBuilder::<EntropyEstimator>::new(entropy_estimator.clone());
    let mut bvcomp = BVComp::<BVGraphModelBuilder<EntropyEstimator>>::new(
        model_builder,
        args.ca.compression_window,
        args.ca.min_interval_length,
        args.ca.max_ref_count,
        0,
    );

    pl.item_name("node")
        .expected_updates(Some(seq_graph.num_nodes()));
    pl.start("Pushing input into the model builder with entropy mock...");

    // second iteration: build a model with the entropy mock writer
    for_![ (_, succ) in seq_graph {
        bvcomp.push(succ)?;
        pl.update();
    }];
    pl.done();

    pl.start("Building the model with entropy mock...");
    // get the final ANSModel4Encoder from the second iteration
    let model4encoder = bvcomp.flush()?.build();
    pl.done();
    let mut bvcomp = BVComp::<BVGraphMeasurableEncoder>::new(
        BVGraphMeasurableEncoder::new(model4encoder, entropy_estimator),
        args.ca.compression_window,
        args.ca.min_interval_length,
        args.ca.max_ref_count,
        0,
    );

    pl.item_name("node")
        .expected_updates(Some(seq_graph.num_nodes()));
    pl.start("Compressing graph...");

    // third iteration: encode with the encoder that uses the ANSModel4Encoder we just got
    for_![ (_, succ) in seq_graph {
        bvcomp.push(succ)?;
        pl.update();
    }];
    pl.done();

    // get phases and the encoder from the bvcomp
    let (encoder, phases) = bvcomp.flush()?.into_inner();
    // get the prelude from the encoder
    let prelude = encoder.into_prelude();

    // let's store what we got
    prelude.mem_dbg(DbgFlags::default() | DbgFlags::PERCENTAGE)?;
    let mut buf = PathBuf::from(&args.new_basename);
    buf.set_extension("ans");
    prelude.store(buf.as_path())?;
    buf.set_extension("phases");
    phases.store(buf.as_path())?;

    Ok(())
}
