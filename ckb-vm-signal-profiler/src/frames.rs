// Note this is largely inspired from
// https://github.com/tikv/pprof-rs/blob/e6b48867b8cc8881ce0b3c0750e409b4c5af91d1/src/frames.rs
use crate::timer::ReportTiming;
use ckb_vm_pprof_protos::profile as protos;
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: Option<String>,
    pub line: Option<u32>,
    pub file: Option<String>,
}

impl Symbol {
    pub fn name(&self) -> String {
        self.name.clone().unwrap_or("<Unknown>".to_owned())
    }

    pub fn line(&self) -> u32 {
        self.line.unwrap_or(0)
    }

    pub fn file(&self) -> String {
        self.file.clone().unwrap_or("<Unknown>".to_owned())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame {
    pub stacks: Vec<Symbol>,
}

#[derive(Default)]
pub struct Report {
    pub data: HashMap<Frame, usize>,
}

const SAMPLES: &str = "samples";
const COUNT: &str = "count";
const CPU: &str = "cpu";
const NANOSECONDS: &str = "nanoseconds";

impl Report {
    pub fn record(&mut self, frame: &Frame) {
        if let Some(c) = self.data.get_mut(frame) {
            *c += 1;
        } else {
            self.data.insert(frame.clone(), 1);
        }
    }

    pub fn pprof(&self, timing: ReportTiming) -> Result<protos::Profile, String> {
        let mut dedup_str = HashSet::new();
        for (frame, _) in self.data.iter() {
            for symbol in frame.stacks.iter() {
                dedup_str.insert(symbol.name());
                dedup_str.insert(symbol.file());
            }
        }
        dedup_str.insert(SAMPLES.into());
        dedup_str.insert(COUNT.into());
        dedup_str.insert(CPU.into());
        dedup_str.insert(NANOSECONDS.into());

        // string table's first element must be an empty string
        let mut str_tbl = vec!["".to_owned()];
        str_tbl.extend(dedup_str.into_iter());

        let mut strings = HashMap::new();
        for (index, name) in str_tbl.iter().enumerate() {
            strings.insert(name.clone(), index);
        }

        let mut samples = vec![];
        let mut loc_tbl = vec![];
        let mut fn_tbl = vec![];
        let mut functions = HashMap::new();
        for (frame, count) in self.data.iter() {
            let mut locs = vec![];
            for symbol in &frame.stacks {
                let name = symbol.name();
                if let Some(loc_idx) = functions.get(&name) {
                    locs.push(*loc_idx);
                    continue;
                }
                let function_id = fn_tbl.len() as u64 + 1;
                let function = protos::Function {
                    id: function_id,
                    name: strings[&name] as i64,
                    // TODO: distinguish between C++ mangled & unmangled names
                    system_name: strings[&name] as i64,
                    filename: strings[&symbol.file()] as i64,
                    ..Default::default()
                };
                functions.insert(name, function_id);
                let line = protos::Line {
                    function_id,
                    line: symbol.line() as i64,
                    ..Default::default()
                };
                let loc = protos::Location {
                    id: function_id,
                    line: vec![line].into(),
                    ..Default::default()
                };
                // the fn_tbl has the same length with loc_tbl
                fn_tbl.push(function);
                loc_tbl.push(loc);
                // current frame locations
                locs.push(function_id);
            }
            let sample = protos::Sample {
                location_id: locs,
                value: vec![*count as i64, *count as i64 * 1_000_000_000 / timing.frequency as i64],
                label: vec![].into(),
                ..Default::default()
            };
            samples.push(sample);
        }
        let samples_value = protos::ValueType {
            field_type: strings[SAMPLES] as i64,
            unit: strings[COUNT] as i64,
            ..Default::default()
        };
        let time_value = protos::ValueType {
            field_type: strings[CPU] as i64,
            unit: strings[NANOSECONDS] as i64,
            ..Default::default()
        };
        let profile = protos::Profile {
            sample_type: vec![samples_value, time_value.clone()].into(),
            sample: samples.into(),
            string_table: str_tbl.into(),
            function: fn_tbl.into(),
            location: loc_tbl.into(),
            time_nanos: timing.start_time.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos() as i64,
            duration_nanos: timing.duration.as_nanos() as i64,
            period_type: Some(time_value).into(),
            period: 1_000_000_000 / timing.frequency as i64,
            ..Default::default()
        };
        Ok(profile)
    }
}
