use ckb_chain_spec::consensus::TYPE_ID_CODE_HASH;
use ckb_hash::blake2b_256;
use ckb_mock_tx_types::{MockResourceLoader, MockTransaction, ReprMockTransaction};
use ckb_script::ScriptGroupType;
use ckb_types::core::{HeaderView, ScriptHashType};
use ckb_types::packed::{Byte32, CellOutput, OutPoint, OutPointVec, Script};
use ckb_types::prelude::{Builder, Entity, Pack};
use ckb_types::H256;
use ckb_vm::Bytes;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct DummyResourceLoader {}

impl MockResourceLoader for DummyResourceLoader {
    fn get_header(&mut self, hash: H256) -> Result<Option<HeaderView>, String> {
        return Err(format!("Header {:x} is missing!", hash));
    }

    fn get_live_cell(&mut self, out_point: OutPoint) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String> {
        return Err(format!("Cell: {:?} is missing!", out_point));
    }
}

pub struct Embed {
    pub data: String,
    pub path: PathBuf,
    pub type_id_dict: HashMap<String, String>,
}

impl Embed {
    pub fn new(path: PathBuf, data: String) -> Self {
        Self { data, path, type_id_dict: HashMap::new() }
    }

    pub fn replace_data(&mut self) -> &mut Self {
        let regex = Regex::new(r"\{\{ ?data (.+?) ?\}\}").unwrap();
        self.data = regex
            .replace_all(&self.data, |caps: &Captures| -> String {
                let cap1 = &caps[1];
                let path = if !Path::new(cap1).is_absolute() {
                    let root = self.path.parent().unwrap();
                    root.join(cap1)
                } else {
                    Path::new(cap1).to_path_buf()
                };
                let data = std::fs::read(&path);
                if data.is_err() {
                    panic!("Read {:?} failed : {:?}", path, data);
                }
                let data = data.unwrap();
                hex::encode(data)
            })
            .to_string();
        self
    }

    pub fn replace_hash(&mut self) -> &mut Self {
        let regex = Regex::new(r"\{\{ ?hash (.+?) ?\}\}").unwrap();
        self.data = regex
            .replace_all(&self.data, |caps: &Captures| -> String {
                let cap1 = &caps[1];
                let path = if !Path::new(cap1).is_absolute() {
                    let root = self.path.parent().unwrap();
                    root.join(cap1)
                } else {
                    Path::new(cap1).to_path_buf()
                };
                let data = std::fs::read(path).unwrap();
                hex::encode(blake2b_256(data))
            })
            .to_string();
        self
    }

    pub fn prelude_type_id(&mut self) -> &mut Self {
        let rule = Regex::new(r"\{\{ ?def_type (.+?) ?\}\}").unwrap();
        for caps in rule.captures_iter(&self.data) {
            let type_id_name = &caps[1];
            assert!(!self.type_id_dict.contains_key(type_id_name));
            let type_id_script = Script::new_builder()
                .args(Bytes::from(type_id_name.to_string()).pack())
                .code_hash(TYPE_ID_CODE_HASH.pack())
                .hash_type(ScriptHashType::Type.into())
                .build();
            let type_id_script_hash = type_id_script.calc_script_hash();
            let type_id_script_hash = format!("{:x}", type_id_script_hash);
            self.type_id_dict.insert(type_id_name.to_string(), type_id_script_hash);
        }
        self
    }

    pub fn replace_def_type(&mut self) -> &mut Self {
        let regex = Regex::new(r#""?\{\{ ?def_type (.+?) ?\}\}"?"#).unwrap();
        self.data = regex
            .replace_all(&self.data, |caps: &Captures| -> String {
                let cap1 = &caps[1];
                let type_id_script_json = ckb_jsonrpc_types::Script {
                    code_hash: TYPE_ID_CODE_HASH,
                    hash_type: ckb_jsonrpc_types::ScriptHashType::Type,
                    args: ckb_jsonrpc_types::JsonBytes::from_vec(cap1.as_bytes().to_vec()),
                };
                return serde_json::to_string_pretty(&type_id_script_json).unwrap();
            })
            .to_string();
        self
    }

    pub fn replace_ref_type(&mut self) -> &mut Self {
        let regex = Regex::new(r"\{\{ ?ref_type (.+?) ?\}\}").unwrap();
        self.data = regex
            .replace_all(&self.data, |caps: &Captures| -> String {
                let cap1 = &caps[1];
                return self.type_id_dict[&cap1.to_string()].clone();
            })
            .to_string();
        self
    }

    pub fn replace_all(&mut self) -> String {
        self.replace_data().replace_hash().prelude_type_id().replace_def_type().replace_ref_type();
        self.data.clone()
    }
}

pub struct HumanReadableCycles(pub u64);

impl std::fmt::Display for HumanReadableCycles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)?;
        if self.0 >= 1024 * 1024 {
            write!(f, "({:.1}M)", self.0 as f64 / 1024. / 1024.)?;
        } else if self.0 >= 1024 {
            write!(f, "({:.1}K)", self.0 as f64 / 1024.)?;
        } else {
        }
        Ok(())
    }
}

// Get script hash by give group type, cell type and cell index.
// Note cell_type should be a string, in the range ["input", "output"].
pub fn get_script_hash_by_index(
    mock_tx: &MockTransaction,
    script_group_type: &ScriptGroupType,
    cell_type: &str,
    cell_index: usize,
) -> Byte32 {
    match (&script_group_type, cell_type) {
        (ScriptGroupType::Lock, "input") => mock_tx.mock_info.inputs[cell_index].output.calc_lock_hash(),
        (ScriptGroupType::Type, "input") => mock_tx.mock_info.inputs[cell_index]
            .output
            .type_()
            .to_opt()
            .expect("cell should have type script")
            .calc_script_hash(),
        (ScriptGroupType::Type, "output") => mock_tx
            .tx
            .raw()
            .outputs()
            .get(cell_index)
            .expect("index out of bound")
            .type_()
            .to_opt()
            .expect("cell should have type script")
            .calc_script_hash(),
        _ => panic!("Invalid specified script: {:?} {} {}", script_group_type, cell_type, cell_index),
    }
}

// Check transactions before executing them to avoid obvious mistakes.
pub fn pre_check(tx: &ReprMockTransaction) -> Result<(), String> {
    let mut mock_cell_deps: Vec<_> = tx.mock_info.cell_deps.iter().map(|c| c.cell_dep.clone()).collect();
    let mut real_cell_deps: Vec<_> = tx.tx.cell_deps.iter().map(|c| c.clone()).collect();
    for dep in &tx.mock_info.cell_deps {
        if dep.cell_dep.dep_type == ckb_jsonrpc_types::DepType::DepGroup {
            let outpoints = OutPointVec::from_slice(dep.data.as_bytes()).unwrap();
            let outpoints: Vec<OutPoint> = outpoints.into_iter().collect();
            let resolved_cell_deps: Vec<_> = outpoints
                .into_iter()
                .map(|o| ckb_jsonrpc_types::CellDep { out_point: o.into(), dep_type: ckb_jsonrpc_types::DepType::Code })
                .collect();
            real_cell_deps.extend(resolved_cell_deps);
        }
    }
    let compare = |a: &ckb_jsonrpc_types::CellDep, b: &ckb_jsonrpc_types::CellDep| {
        let l = serde_json::to_string(a).unwrap();
        let r = serde_json::to_string(b).unwrap();
        l.cmp(&r)
    };
    mock_cell_deps.sort_by(compare);
    real_cell_deps.sort_by(compare);
    if mock_cell_deps != real_cell_deps {
        return Err(String::from("Precheck: celldeps is mismatched"));
    }
    let mock_inputs: Vec<_> = tx.mock_info.inputs.iter().map(|i| i.input.clone()).collect();
    let real_inputs: Vec<_> = tx.tx.inputs.clone();
    if mock_inputs != real_inputs {
        return Err(String::from("Precheck: inputs is mismatched"));
    }
    let mock_header_deps: Vec<_> = tx.mock_info.header_deps.iter().map(|h| h.hash.clone()).collect();
    let read_header_deps: Vec<_> = tx.tx.header_deps.clone();
    if mock_header_deps != read_header_deps {
        return Err(String::from("Precheck: header deps is mismatched"));
    }
    Ok(())
}
