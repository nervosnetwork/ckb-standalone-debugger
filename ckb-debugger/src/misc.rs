use ckb_mock_tx_types::{MockResourceLoader, MockTransaction};
use ckb_script::ScriptGroupType;
use ckb_types::H256;
use ckb_types::core::HeaderView;
use ckb_types::packed::{Byte32, CellOutput, OutPoint};
use ckb_vm::Bytes;
use ckb_vm_syscall_tracer::CollectorKey;
use ckb_vm_syscall_tracer::generated::traces::VmCreation;
use std::collections::HashMap;

pub struct DummyResourceLoader {}

impl MockResourceLoader for DummyResourceLoader {
    fn get_header(&mut self, hash: H256) -> Result<Option<HeaderView>, String> {
        return Err(format!("Header {:x} is missing!", hash));
    }

    fn get_live_cell(&mut self, out_point: OutPoint) -> Result<Option<(CellOutput, Bytes, Option<Byte32>)>, String> {
        return Err(format!("Cell: {:?} is missing!", out_point));
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

pub fn collector_key_str(collector_key: &CollectorKey) -> String {
    if collector_key.generation_id != 0 {
        format!("{}/{}", collector_key.vm_id, collector_key.generation_id)
    } else {
        format!("{}", collector_key.vm_id)
    }
}

// Recursive helper function to print the tree.
pub fn print_vm_tree_recursive(
    tree: &HashMap<CollectorKey, Vec<VmCreation>>,
    hint: &HashMap<CollectorKey, String>,
    ckey: CollectorKey,
    prefix: &str,
    is_last: bool,
) {
    let mut line = format!("Spawn tree: {}{}", prefix, collector_key_str(&ckey));
    if line.chars().count() < 32 {
        line.push_str(String::from(" ").repeat(32 - line.chars().count()).as_str());
    }
    line.push_str(" ");
    line.push_str(&hint.get(&ckey).unwrap());
    println!("{}", line);

    // Get children, if any
    if let Some(children) = tree.get(&ckey) {
        // Update prefix for children
        let new_prefix = if prefix.is_empty() {
            "".to_string()
        } else if is_last {
            format!("{}    ", prefix.trim_end_matches("├── ").trim_end_matches("└── "))
        } else {
            format!("{}│   ", prefix.trim_end_matches("├── ").trim_end_matches("└── "))
        };
        // Print each child
        for (i, child) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;
            let child_prefix =
                if is_last_child { format!("{}└── ", new_prefix) } else { format!("{}├── ", new_prefix) };
            print_vm_tree_recursive(
                tree,
                hint,
                CollectorKey { vm_id: child.vm_id, generation_id: child.generation_id },
                &child_prefix,
                is_last_child,
            );
        }
    }
}
