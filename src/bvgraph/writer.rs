use std::{convert::Infallible};
use std::marker::PhantomData;
use webgraph::graph::bvgraph::BVGraphCodesWriter;

use crate::bvgraph::Component;
use crate::multi_model_ans::encoder::ANSCompressorPhase;
use crate::{
    multi_model_ans::{
        encoder::ANSEncoder, model4encoder::ANSModel4Encoder,
        model4encoder_builder::ANSModel4EncoderBuilder,
    },
};
use crate::bvgraph::mock_writers::{EntropyMockWriter, len, Log2MockWriter, MockWriter};
use crate::utils::ans_utilities::get_symbol_costs_table;


/// A [`BVGraphCodesWriter`] that builds an [`ANSModel4Encoder`] using the
/// symbols written to it.
///
/// Note that a [`BVGraphCodesWriter`] needs a mock writer to measure code
/// lengths. We use a [`Log2MockWriter`] that returns `⌊log₂(x)⌋` as the number
/// of bits written encoding `x`.
pub struct BVGraphModelBuilder<const FIDELITY: usize, const RADIX: usize, MW>
where
    MW: BVGraphCodesWriter + MockWriter,
{
    model_builder: ANSModel4EncoderBuilder<FIDELITY, RADIX>,
    symbol_costs_table: Vec<Vec<usize>>,
    _marker: PhantomData<MW>,
}

impl<const FIDELITY: usize, const RADIX: usize, MW> BVGraphModelBuilder<FIDELITY, RADIX, MW>
where
    MW: BVGraphCodesWriter + MockWriter,
{
    pub fn new(symbol_costs_table: Vec<Vec<usize>>) -> Self {
        Self {
            model_builder: ANSModel4EncoderBuilder::<FIDELITY, RADIX>::new(9),
            symbol_costs_table,
            _marker: PhantomData,
        }
    }

    /// Build an [`ANSModel4Encoder`] from the symbols written to this
    /// [`BVGraphModelBuilder`].
    pub fn build(self) -> ANSModel4Encoder {
        self.model_builder.build()
    }
}

impl<const FIDELITY: usize, const RADIX: usize, MW> BVGraphCodesWriter for BVGraphModelBuilder<FIDELITY, RADIX, MW>
where
    MW: BVGraphCodesWriter + MockWriter,
{
    type Error = Infallible;

    type MockWriter = MW;

    fn mock(&self) -> Self::MockWriter {
        MW::build(self.symbol_costs_table.clone()) // TODO: now it's a clone
    }

    fn write_outdegree(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::Outdegree as usize);
        len(value)
    }

    fn write_reference_offset(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::ReferenceOffset as usize);
        len(value)
    }

    fn write_block_count(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::BlockCount as usize);
        len(value)
    }

    fn write_blocks(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::Blocks as usize);
        len(value)
    }

    fn write_interval_count(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::IntervalCount as usize);
        len(value)
    }

    fn write_interval_start(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::IntervalStart as usize);
        len(value)
    }

    fn write_interval_len(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::IntervalLen as usize);
        len(value)
    }

    fn write_first_residual(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::FirstResidual as usize);
        len(value)
    }

    fn write_residual(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.model_builder
            .push_symbol(value, Component::Residual as usize);
        len(value)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}


/// A [`BVGraphCodesWriter`] that writes to an [`ANSEncoder`].
///
/// Data is gathered in a number of buffers, one for each [component](`Component`).
/// At the next node (i.e. when `write_outdegree` is called again), the buffers
/// are emptied in reverse order.
pub struct BVGraphWriter<const FIDELITY: usize, const RADIX: usize> {
    /// The container containing the buffers (one for each [component](`Component`)) where symbols are collected.
    data: [Vec<usize>; 9],

    /// The index of the node the encoder is currently encoding.
    curr_node: usize,

    /// The encoder used by this writer to encode symbols.
    encoder: ANSEncoder<FIDELITY, RADIX>,

    /// A buffer containing a [`ANSCompressorPhase`], one for each node.
    phases: Vec<ANSCompressorPhase>,

    mock_writer: EntropyMockWriter,
}

impl<const FIDELITY: usize, const RADIX: usize> BVGraphWriter<FIDELITY, RADIX> {
    pub fn new(model: ANSModel4Encoder) -> Self {
        let encoder = ANSEncoder::<FIDELITY, RADIX>::new(model);
        let table = get_symbol_costs_table(
            &encoder.model.tables,
            &encoder.model.frame_sizes,
            FIDELITY,
            RADIX
        );

        Self {
            curr_node: usize::MAX,
            data: [
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
            encoder,
            phases: Vec::new(),
            mock_writer: EntropyMockWriter::build(table),
        }
    }

    /// Consume self and return the encoder.
    pub fn into_inner(self, ) -> (ANSEncoder<FIDELITY, RADIX>, Vec<ANSCompressorPhase>) {
        (self.encoder, self.phases)
    }
}

impl<const FIDELITY: usize, const RADIX: usize> BVGraphCodesWriter for BVGraphWriter<FIDELITY, RADIX> {
    type Error = Infallible;

    type MockWriter = EntropyMockWriter;

    fn mock(&self) -> Self::MockWriter { // do i have to return a mock here?
        let table = get_symbol_costs_table(
            &self.encoder.model.tables,
            &self.encoder.model.frame_sizes,
            FIDELITY,
            RADIX
        );
        EntropyMockWriter::build(table)
    }

    fn write_outdegree(&mut self, value: u64) -> Result<usize, Self::Error> {
        if self.curr_node != usize::MAX {
            for (component, symbols) in self.data
                [Component::FirstResidual as usize..=Component::Residual as usize]
                .iter()
                .enumerate()
                .rev()
            {
                for &symbol in symbols.iter().rev() {
                    self.encoder
                        .encode(symbol as u64, component + Component::FirstResidual as usize);
                }
            }

            debug_assert_eq!(
                self.data[Component::IntervalStart as usize].len(),
                self.data[Component::IntervalLen as usize].len()
            );

            for i in (0..self.data[Component::IntervalStart as usize].len()).rev() {
                self.encoder.encode(
                    self.data[Component::IntervalLen as usize][i] as u64,
                    Component::IntervalLen as usize,
                );
                self.encoder.encode(
                    self.data[Component::IntervalStart as usize][i] as u64,
                    Component::IntervalStart as usize,
                );
            }

            for (component, symbols) in self.data
                [Component::Outdegree as usize..=Component::IntervalCount as usize]
                .iter()
                .enumerate()
                .rev()
            {
                for &symbol in symbols.iter().rev() {
                    self.encoder.encode(symbol as u64, component);
                }
            }
            // save state of the encoder as soon as it finishes encoding the node
            self.phases
                .push(self.encoder.get_current_compressor_phase());
        }

        // Increase and cleanup
        self.curr_node = self.curr_node.wrapping_add(1);
        for symbols in &mut self.data {
            symbols.clear();
        }

        self.data[Component::Outdegree as usize].push(value as usize);
        self.mock_writer.write_outdegree(value)
    }

    fn write_reference_offset(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::ReferenceOffset as usize].push(value as usize);
        self.mock_writer.write_reference_offset(value)
    }

    fn write_block_count(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::BlockCount as usize].push(value as usize);
        self.mock_writer.write_block_count(value)
    }

    fn write_blocks(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::Blocks as usize].push(value as usize);
        self.mock_writer.write_blocks(value)
    }

    fn write_interval_count(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::IntervalCount as usize].push(value as usize);
        self.mock_writer.write_interval_count(value)
    }

    fn write_interval_start(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::IntervalStart as usize].push(value as usize);
        self.mock_writer.write_interval_start(value)
    }

    fn write_interval_len(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::IntervalLen as usize].push(value as usize);
        self.mock_writer.write_interval_len(value)
    }

    fn write_first_residual(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::FirstResidual as usize].push(value as usize);
        self.mock_writer.write_first_residual(value)
    }

    fn write_residual(&mut self, value: u64) -> Result<usize, Self::Error> {
        self.data[Component::Residual as usize].push(value as usize);
        self.mock_writer.write_residual(value)
    }

    // Dump last node
    fn flush(&mut self) -> Result<(), Self::Error> {
        for (component, symbols) in self.data.iter().enumerate().rev() {
            for &symbol in symbols.iter().rev() {
                self.encoder.encode(symbol as u64, component);
            }
        }
        self.phases
            .push(self.encoder.get_current_compressor_phase());
        Ok(())
    }
}