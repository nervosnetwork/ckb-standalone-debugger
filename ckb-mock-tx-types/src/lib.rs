// Originally modified from https://github.com/nervosnetwork/ckb-cli/blob/d6eceb3f9f108a17bcae0b1d760023e5da1e6e6a/ckb-sdk-types/src/transaction.rs
// Current version modified(just fix clippy) from https://github.com/nervosnetwork/ckb-standalone-debugger/blob/5c0490cb8279f7c7860c403b9f1773b65403e57a/src/transaction.rs
use ckb_jsonrpc_types as json_types;
use ckb_traits::{CellDataProvider, HeaderProvider};
use ckb_types::{
    bytes::Bytes,
    core::{
        cell::{CellMeta, CellMetaBuilder, CellProvider, CellStatus, HeaderChecker},
        error::OutPointError,
        DepType, EpochNumberWithFraction, HeaderView, TransactionInfo, TransactionView,
    },
    packed::{Byte32, CellDep, CellInput, CellOutput, OutPoint, OutPointVec, Transaction},
    prelude::*,
    H256,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct MockCellDep {
    pub cell_dep: CellDep,
    pub output: CellOutput,
    pub data: Bytes,
    pub header: Option<Byte32>,
}

#[derive(Clone, Default)]
pub struct MockInput {
    pub input: CellInput,
    pub output: CellOutput,
    pub data: Bytes,
    pub header: Option<Byte32>,
}

#[derive(Clone, Default)]
pub struct MockInfo {
    pub inputs: Vec<MockInput>,
    pub cell_deps: Vec<MockCellDep>,
    pub header_deps: Vec<HeaderView>,
}

/// A wrapper transaction with mock inputs and deps
#[derive(Clone, Default)]
pub struct MockTransaction {
    pub mock_info: MockInfo,
    pub tx: Transaction,
}

impl MockTransaction {
    pub fn get_input_cell<F: FnMut(OutPoint) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String>>(
        &self,
        input: &CellInput,
        mut live_cell_getter: F,
    ) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String> {
        for mock_input in &self.mock_info.inputs {
            if input == &mock_input.input {
                return Ok(Some((
                    mock_input.output.clone(),
                    mock_input.data.clone(),
                    mock_input.header.clone(),
                )));
            }
        }
        live_cell_getter(input.previous_output())
    }

    pub fn get_dep_cell<F: FnMut(OutPoint) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String>>(
        &self,
        out_point: &OutPoint,
        mut live_cell_getter: F,
    ) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String> {
        for mock_cell in &self.mock_info.cell_deps {
            if out_point == &mock_cell.cell_dep.out_point() {
                return Ok(Some((
                    mock_cell.output.clone(),
                    mock_cell.data.clone(),
                    mock_cell.header.clone(),
                )));
            }
        }
        live_cell_getter(out_point.clone())
    }

    pub fn get_header<F: FnMut(H256) -> Result<Option<HeaderView>, String>>(
        &self,
        block_hash: &H256,
        mut header_getter: F,
    ) -> Result<Option<HeaderView>, String> {
        for mock_header in &self.mock_info.header_deps {
            if block_hash == &mock_header.hash().unpack() {
                return Ok(Some(mock_header.clone()));
            }
        }
        header_getter(block_hash.clone())
    }

    /// Generate the core transaction
    pub fn core_transaction(&self) -> TransactionView {
        self.tx.clone().into_view()
    }
}

pub trait MockResourceLoader {
    fn get_header(&mut self, hash: H256) -> Result<Option<HeaderView>, String>;
    fn get_live_cell(&mut self, out_point: OutPoint) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String>;
}

#[derive(Clone)]
pub struct Resource {
    required_cells: HashMap<OutPoint, CellMeta>,
    required_headers: HashMap<Byte32, HeaderView>,
}

impl Resource {
    pub fn from_mock_tx(mock_tx: &MockTransaction) -> Result<Resource, String> {
        Self::from_both(mock_tx, DummyResourceLoader {})
    }

    #[allow(clippy::mutable_key_type)]
    pub fn from_both<L: MockResourceLoader>(mock_tx: &MockTransaction, mut loader: L) -> Result<Resource, String> {
        let tx = mock_tx.core_transaction();
        let mut required_cells = HashMap::default();
        let mut required_headers = HashMap::default();

        for input in tx.inputs().into_iter() {
            let (output, data, header) = mock_tx
                .get_input_cell(&input, |out_point| loader.get_live_cell(out_point))?
                .ok_or_else(|| format!("Can not get CellOutput by input={}", input))?;
            let cell_meta = CellMetaBuilder::from_cell_output(output, data)
                .out_point(input.previous_output())
                .transaction_info(Self::build_transaction_info(header))
                .build();
            required_cells.insert(input.previous_output(), cell_meta);
        }

        for cell_dep in tx.cell_deps().into_iter() {
            let (output, data, header) = mock_tx
                .get_dep_cell(&cell_dep.out_point(), |out_point| loader.get_live_cell(out_point))?
                .ok_or_else(|| format!("Can not get CellOutput by dep={}", cell_dep))?;
            // Handle dep group
            if cell_dep.dep_type() == DepType::DepGroup.into() {
                for sub_out_point in OutPointVec::from_slice(&data)
                    .map_err(|err| format!("Parse dep group data error: {}", err))?
                    .into_iter()
                {
                    let (sub_output, sub_data, sub_header) = mock_tx
                        .get_dep_cell(&sub_out_point, |out_point| loader.get_live_cell(out_point))?
                        .ok_or_else(|| format!("(dep group) Can not get CellOutput by out_point={}", sub_out_point))?;

                    let sub_cell_meta = CellMetaBuilder::from_cell_output(sub_output, sub_data)
                        .out_point(sub_out_point.clone())
                        .transaction_info(Self::build_transaction_info(sub_header))
                        .build();
                    required_cells.insert(sub_out_point, sub_cell_meta);
                }
            }
            let cell_meta = CellMetaBuilder::from_cell_output(output, data)
                .out_point(cell_dep.out_point())
                .transaction_info(Self::build_transaction_info(header))
                .build();
            required_cells.insert(cell_dep.out_point(), cell_meta);
        }

        for block_hash in tx.header_deps().into_iter() {
            let header = mock_tx
                .get_header(&block_hash.unpack(), |block_hash| loader.get_header(block_hash))?
                .ok_or_else(|| format!("Can not get header: {:x}", block_hash))?;
            required_headers.insert(block_hash, header);
        }

        Ok(Resource {
            required_cells,
            required_headers,
        })
    }

    fn build_transaction_info(header: Option<Byte32>) -> TransactionInfo {
        // Only block hash might be used by script syscalls
        TransactionInfo::new(
            0,
            EpochNumberWithFraction::new(0, 0, 1800),
            header.unwrap_or_default(),
            0,
        )
    }
}

impl HeaderChecker for Resource {
    fn check_valid(&self, block_hash: &Byte32) -> Result<(), OutPointError> {
        if !self.required_headers.contains_key(block_hash) {
            return Err(OutPointError::InvalidHeader(block_hash.clone()));
        }
        Ok(())
    }
}

impl CellProvider for Resource {
    fn cell(&self, out_point: &OutPoint, _with_data: bool) -> CellStatus {
        self.required_cells.get(out_point).cloned().map(CellStatus::live_cell).unwrap_or(CellStatus::Unknown)
    }
}

impl CellDataProvider for Resource {
    fn get_cell_data(&self, out_point: &OutPoint) -> Option<Bytes> {
        self.required_cells.get(out_point).and_then(|cell_meta| cell_meta.mem_cell_data.clone())
    }

    fn get_cell_data_hash(&self, out_point: &OutPoint) -> Option<Byte32> {
        self.required_cells.get(out_point).and_then(|cell_meta| cell_meta.mem_cell_data_hash.clone())
    }
}

impl HeaderProvider for Resource {
    fn get_header(&self, block_hash: &Byte32) -> Option<HeaderView> {
        self.required_headers.get(block_hash).cloned()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReprMockCellDep {
    pub cell_dep: json_types::CellDep,
    pub output: json_types::CellOutput,
    pub data: json_types::JsonBytes,
    pub header: Option<H256>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ReprMockInput {
    pub input: json_types::CellInput,
    pub output: json_types::CellOutput,
    pub data: json_types::JsonBytes,
    pub header: Option<H256>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ReprMockInfo {
    pub inputs: Vec<ReprMockInput>,
    pub cell_deps: Vec<ReprMockCellDep>,
    pub header_deps: Vec<json_types::HeaderView>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ReprMockTransaction {
    pub mock_info: ReprMockInfo,
    pub tx: json_types::Transaction,
}

impl From<MockCellDep> for ReprMockCellDep {
    fn from(dep: MockCellDep) -> ReprMockCellDep {
        ReprMockCellDep {
            cell_dep: dep.cell_dep.into(),
            output: dep.output.into(),
            data: json_types::JsonBytes::from_bytes(dep.data),
            header: dep.header.map(|h| h.unpack()),
        }
    }
}
impl From<ReprMockCellDep> for MockCellDep {
    fn from(dep: ReprMockCellDep) -> MockCellDep {
        MockCellDep {
            cell_dep: dep.cell_dep.into(),
            output: dep.output.into(),
            data: dep.data.into_bytes(),
            header: dep.header.map(|h| h.pack()),
        }
    }
}

impl From<MockInput> for ReprMockInput {
    fn from(input: MockInput) -> ReprMockInput {
        ReprMockInput {
            input: input.input.into(),
            output: input.output.into(),
            data: json_types::JsonBytes::from_bytes(input.data),
            header: input.header.map(|h| h.unpack()),
        }
    }
}
impl From<ReprMockInput> for MockInput {
    fn from(input: ReprMockInput) -> MockInput {
        MockInput {
            input: input.input.into(),
            output: input.output.into(),
            data: input.data.into_bytes(),
            header: input.header.map(|h| h.pack()),
        }
    }
}

impl From<MockInfo> for ReprMockInfo {
    fn from(info: MockInfo) -> ReprMockInfo {
        ReprMockInfo {
            inputs: info.inputs.into_iter().map(Into::into).collect(),
            cell_deps: info.cell_deps.into_iter().map(Into::into).collect(),
            header_deps: info
                .header_deps
                .into_iter()
                .map(|header| {
                    // Keep the user given hash
                    let hash = header.hash().unpack();
                    let mut json_header: json_types::HeaderView = header.into();
                    json_header.hash = hash;
                    json_header
                })
                .collect(),
        }
    }
}

impl From<ReprMockInfo> for MockInfo {
    fn from(info: ReprMockInfo) -> MockInfo {
        MockInfo {
            inputs: info.inputs.into_iter().map(Into::into).collect(),
            cell_deps: info.cell_deps.into_iter().map(Into::into).collect(),
            header_deps: info
                .header_deps
                .into_iter()
                .map(|json_header| {
                    // Keep the user given hash
                    let hash = json_header.hash.pack();
                    HeaderView::from(json_header).fake_hash(hash)
                })
                .collect(),
        }
    }
}

impl From<MockTransaction> for ReprMockTransaction {
    fn from(tx: MockTransaction) -> ReprMockTransaction {
        ReprMockTransaction {
            mock_info: tx.mock_info.into(),
            tx: tx.tx.into(),
        }
    }
}
impl From<ReprMockTransaction> for MockTransaction {
    fn from(tx: ReprMockTransaction) -> MockTransaction {
        MockTransaction {
            mock_info: tx.mock_info.into(),
            tx: tx.tx.into(),
        }
    }
}

pub struct DummyResourceLoader {}

impl MockResourceLoader for DummyResourceLoader {
    fn get_header(&mut self, hash: H256) -> Result<Option<HeaderView>, String> {
        Err(format!("Header {:x} is missing!", hash))
    }

    fn get_live_cell(&mut self, out_point: OutPoint) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String> {
        Err(format!("Cell: {:?} is missing!", out_point))
    }
}
