use criterion::{criterion_group, BatchSize, Criterion};
use dsi_bitstream::prelude::BE;
use folded_streaming_rans::bvgraph::mock_writers::{EntropyEstimator, Log2Estimator};
use folded_streaming_rans::bvgraph::writer::{BVGraphMeasurableEncoder, BVGraphModelBuilder};
use pprof::criterion::{Output, PProfProfiler};
use webgraph::graphs::{BVComp, BVGraph};
use webgraph::prelude::SequentialLabeling;

fn encoding_bench(c: &mut Criterion) {
    let graph = BVGraph::with_basename("tests/data/cnr-2000/cnr-2000")
        .endianness::<BE>()
        .load()
        .unwrap();

    let log2_mock = Log2Estimator::new();
    let model_builder = BVGraphModelBuilder::<Log2Estimator>::new(log2_mock);
    let mut bvcomp = BVComp::<BVGraphModelBuilder<Log2Estimator>>::new(model_builder, 7, 2, 3, 0);

    // First iteration with Log2MockWriter
    bvcomp.extend(graph.iter()).unwrap();

    let model4encoder = bvcomp.flush().unwrap().build();
    let folding_params = model4encoder.get_folding_params();
    let entropic_mock = EntropyEstimator::new(&model4encoder, folding_params);
    let model_builder = BVGraphModelBuilder::<EntropyEstimator>::new(entropic_mock.clone());
    let mut bvcomp =
        BVComp::<BVGraphModelBuilder<EntropyEstimator>>::new(model_builder, 7, 2, 3, 0);

    // second iteration with EntropyMockWriter
    bvcomp.extend(graph.iter()).unwrap();

    let model4encoder = bvcomp.flush().unwrap().build();

    let mut group = c.benchmark_group("encoding");
    group.measurement_time(std::time::Duration::from_secs(100));
    group.sample_size(20);

    group.bench_function("cnr-2000", |b| {
        b.iter_batched(
            || {
                BVComp::<BVGraphMeasurableEncoder>::new(
                    BVGraphMeasurableEncoder::new(model4encoder.clone(), entropic_mock.clone()),
                    7,
                    2,
                    3,
                    0,
                )
            },
            |mut bvcomp| bvcomp.extend(graph.iter()).unwrap(),
            BatchSize::SmallInput,
        )
    });
    group.finish()
}

criterion_group! {
    name = encoder_benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = encoding_bench
}
