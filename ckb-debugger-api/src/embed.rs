use ckb_chain_spec::consensus::TYPE_ID_CODE_HASH;
use ckb_hash::blake2b_256;
use ckb_types::core::ScriptHashType;
use ckb_types::packed::Script;
use ckb_types::prelude::{Builder, Entity, Pack};
use ckb_vm::Bytes;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
