mod utils;

use folded_streaming_rans::bvgraph::BVGraphComponent;
use folded_streaming_rans::multi_model_ans::decoder::ANSDecoder;
use folded_streaming_rans::multi_model_ans::encoder::ANSEncoder;
use folded_streaming_rans::multi_model_ans::model4decoder::ANSModel4Decoder;
use folded_streaming_rans::multi_model_ans::model4encoder_builder::ANSModel4EncoderBuilder;
use folded_streaming_rans::RawSymbol;

#[test]
fn decoder_decodes_correctly_single_dummy_sequence() {
    let source = vec![1_u64, 1, 1, 2, 2, 2, 3, 3, 4, 5];
    let mut model4encoder_builder = ANSModel4EncoderBuilder::new();

    for symbol in &source {
        model4encoder_builder
            .push_symbol(*symbol, BVGraphComponent::Outdegree)
            .unwrap();
    }

    let encoder_model = model4encoder_builder.build();
    let mut encoder = ANSEncoder::new(encoder_model);

    for symbol in &source {
        encoder.encode(*symbol, BVGraphComponent::Outdegree);
    }

    let prelude = encoder.into_prelude();
    let model = ANSModel4Decoder::new(&prelude.tables);
    let mut decoder = ANSDecoder::new(&prelude, &model);
    let mut decoded_symbols: Vec<RawSymbol> = Vec::new();

    for _ in 0..source.len() {
        decoded_symbols.push(decoder.decode(BVGraphComponent::Outdegree));
    }
    decoded_symbols.reverse(); // since encodes as a LIFO

    assert_eq!(decoded_symbols, source);
}

/*
#[test]
fn decoder_decodes_correctly_dummy_sequence_with_folding() {
    let source = vec![1000, 1000, 2000];

    let mut model4encoder_builder = ANSModel4EncoderBuilder::new(COMPONENT_ARGS);

    for symbol in &source {
        model4encoder_builder.push_symbol(*symbol, BVGraphComponent::Outdegree).unwrap();
    }

    let encoder_model = model4encoder_builder.build();
    let mut encoder = ANSEncoder::new(encoder_model);

    for symbol in &source {
        encoder.encode(*symbol, BVGraphComponent::Outdegree);
    }

    let prelude = encoder.into_prelude();
    let model = ANSModel4Decoder::new(&prelude.tables);
    let mut decoder = ANSDecoder::new(&prelude, &model);
    let mut decoded_symbols: Vec<RawSymbol> = Vec::new();

    for _ in 0..source.len() {
        decoded_symbols.push(decoder.decode(BVGraphComponent::Outdegree));
    }

    decoded_symbols.reverse(); // since encodes as a LIFO

    assert_eq!(decoded_symbols, source);
}

#[test]
fn decoder_decodes_correctly_real_sequence() {
    let source = get_zipfian_distr(0, 1.2).to_vec();

    let mut model4encoder_builder = ANSModel4EncoderBuilder::new(COMPONENT_ARGS);

    for symbol in &source {
        model4encoder_builder.push_symbol(*symbol, BVGraphComponent::Outdegree).unwrap();
    }

    let encoder_model = model4encoder_builder.build();
    let mut encoder = ANSEncoder::new(encoder_model);

    for symbol in &source {
        encoder.encode(*symbol, BVGraphComponent::Outdegree);
    }

    let prelude = encoder.into_prelude();
    let model = ANSModel4Decoder::new(&prelude.tables);
    let mut decoder = ANSDecoder::new(&prelude, &model);
    let mut decoded_symbols: Vec<RawSymbol> = Vec::new();

    for _ in 0..source.len() {
        decoded_symbols.push(decoder.decode(BVGraphComponent::Outdegree));
    }

    decoded_symbols.reverse(); // since encodes as a LIFO

    assert_eq!(decoded_symbols, source);
}

#[test]
fn decoder_decodes_correctly_dummy_sequences() {
    let first_source = vec![1_u64, 1, 1, 2, 2, 2, 3, 3, 4, 5];
    let second_source = vec![1_u64, 3, 3, 3, 2, 2, 3, 3, 4, 5];
    let mut encoder_model_builder = ANSModel4EncoderBuilder::new(COMPONENT_ARGS);

    for index in 0..first_source.len() {
        encoder_model_builder.push_symbol(first_source[index], BVGraphComponent::Outdegree).unwrap();
        encoder_model_builder.push_symbol(second_source[index], BVGraphComponent::BlockCount).unwrap();
    }

    let encoder_model = encoder_model_builder.build();
    let mut encoder = ANSEncoder::new(encoder_model);

    for index in 0..first_source.len() {
        encoder.encode(first_source[index], BVGraphComponent::Outdegree);
        encoder.encode(second_source[index], BVGraphComponent::BlockCount);
    }

    let prelude = encoder.into_prelude();
    let model = ANSModel4Decoder::new(&prelude.tables);
    let mut decoder = ANSDecoder::new(&prelude, &model);

    let mut first_decoded_sequence: Vec<RawSymbol> = Vec::new();
    let mut second_decoded_sequence: Vec<RawSymbol> = Vec::new();

    for _ in 0..first_source.len() {
        second_decoded_sequence.push(decoder.decode(BVGraphComponent::BlockCount)); // let's start from the last encoded
        first_decoded_sequence.push(decoder.decode(BVGraphComponent::Outdegree));
    }

    first_decoded_sequence.reverse(); // since encodes as a LIFO
    second_decoded_sequence.reverse();

    assert_eq!(first_decoded_sequence, first_source);
    assert_eq!(second_decoded_sequence, second_source);
}

#[test]
// Frame sizes: [9, 14, 13, 10] (note that these are actually log_2 of the frame sizes)
fn decoder_decodes_correctly_real_interleaved_sequences_with_different_frame_sizes() {
    // let's get a random sequence of symbols to encode and map them to have this shape: (component, symbol)
    let first_sequence = get_zipfian_distr(0, 1.3)
        .iter()
        .map(|symbol| (BVGraphComponent::Outdegree, *symbol))
        .collect::<Vec<(BVGraphComponent, RawSymbol)>>()[..SYMBOL_LIST_LENGTH/2000]
        .to_vec();

    let second_sequence = get_zipfian_distr(1, 1.2)
        .iter()
        .map(|symbol| (BVGraphComponent::BlockCount, *symbol))
        .collect::<Vec<(BVGraphComponent, RawSymbol)>>();

    let third_sequence = get_zipfian_distr(2, 1.0)
        .iter()
        .map(|symbol| (BVGraphComponent::Residual, *symbol))
        .collect::<Vec<(BVGraphComponent, RawSymbol)>>();

    // now let's unify each source in a single one and randomize it
    let mut source = vec![first_sequence, second_sequence, third_sequence].concat();
    source.shuffle(&mut rand::thread_rng());

    let mut model4encoder_builder = ANSModel4EncoderBuilder::new(COMPONENT_ARGS);

    for (component, symbol) in &source {
        model4encoder_builder.push_symbol(*symbol, *component).unwrap();
    }

    let encoder_model = model4encoder_builder.build();
    let mut encoder = ANSEncoder::new(encoder_model);
    let mut expected = vec![Vec::new(); BVGraphComponent::COMPONENTS];

    for (component, symbol) in &source {
        expected[*component as usize].push(*symbol);
    }

    // now encode each symbol with the corresponding model previously associated
    for (component, symbol) in source.iter() {
        encoder.encode(*symbol, *component);
    }

    let prelude = encoder.into_prelude();
    let model = ANSModel4Decoder::new(&prelude.tables);
    let mut decoder = ANSDecoder::new(&prelude, &model);
    let mut decoded: Vec<Vec<RawSymbol>> = vec![Vec::new(); BVGraphComponent::COMPONENTS];

    source.reverse(); // now let's reverse the order of the model_index-symbol pairs to decode in reverse

    for (component, _symbol) in &source {
        decoded[*component as usize].push(decoder.decode(*component));
    }

    decoded.iter_mut().for_each(|sequence| sequence.reverse()); // they have been decoded in reversed order

    assert_eq!(expected[0], decoded[0]);
    assert_eq!(expected[1], decoded[1]);
    assert_eq!(expected[2], decoded[2]);
    assert_eq!(expected[3], decoded[3]);
}
*/
