use ckb_vm_pprof_protos::profile;
use clap::{arg, command, value_parser};
use protobuf::Message;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: Option<String>,
    pub file: Option<String>,
}

impl Symbol {
    pub fn name(&self) -> String {
        self.name.clone().unwrap_or("<Unknown>".to_owned())
    }

    pub fn file(&self) -> String {
        self.file.clone().unwrap_or("<Unknown>".to_owned())
    }
}

struct Frame {
    stack: Vec<Symbol>,
    cycles: u64,
}

const CYCLES: &str = "cycles";
const COUNT: &str = "count";
const CPU: &str = "fakecpu";
const NANOSECONDS: &str = "nanoseconds";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = command!()
        .arg(arg!(--"output-file" <VALUE> "Output file path to generate").default_value("output.pprof"))
        .arg(arg!(--"input-file" <VALUE> "Input file path, use '-' to denote stdin").default_value("-"))
        .arg(arg!(--"frequency" <VALUE> "Frequency to use, default value is 0.5Ghz, meaning 1 CKB cycle takes roughly 2 nanoseconds to run, which resembles on real stats gathered on CKB mainnet")
                .default_value("500000000").value_parser(value_parser!(i64)))
        .get_matches();

    let output_file = matches.get_one::<String>("output-file").expect("output file");
    let input_file = matches.get_one::<String>("input-file").expect("input file");
    let frequency = *matches.get_one::<i64>("frequency").expect("frequency");

    let mut frames = Vec::new();

    let lines: Box<dyn Iterator<Item = Result<String, io::Error>>> = if input_file == "-" {
        Box::new(io::stdin().lines())
    } else {
        let file = File::open(&input_file).expect("open file");
        Box::new(BufReader::new(file).lines())
    };

    for line in lines {
        let line = line?;
        let i = line.rfind(" ").expect("no cycles available!");

        let mut stack: Vec<Symbol> = line[0..i]
            .split("; ")
            .map(|s| match s.find(":") {
                Some(j) => Symbol {
                    file: Some(s[0..j].to_string()),
                    name: Some(normalize_function_name(&s[j + 1..s.len()])),
                },
                None => Symbol {
                    name: Some(normalize_function_name(s)),
                    file: None,
                },
            })
            .collect();
        stack.reverse();
        let cycles = u64::from_str(&line[i + 1..line.len()]).expect("invalid cycle");

        frames.push(Frame { stack, cycles });
    }

    let mut dedup_str: HashSet<String> = HashSet::new();
    for Frame { stack, .. } in &frames {
        for symbol in stack {
            dedup_str.insert(symbol.name());
            dedup_str.insert(symbol.file());
        }
    }

    dedup_str.insert(CYCLES.into());
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
    for Frame { stack, cycles } in &frames {
        let mut locs = vec![];
        for symbol in stack {
            let name = symbol.name();
            if let Some(loc_idx) = functions.get(&name) {
                locs.push(*loc_idx);
                continue;
            }
            let function_id = fn_tbl.len() as u64 + 1;
            let function = profile::Function {
                id: function_id,
                name: strings[&name] as i64,
                // TODO: distinguish between C++ mangled & unmangled names
                system_name: strings[&name] as i64,
                filename: strings[&symbol.file()] as i64,
                ..Default::default()
            };
            functions.insert(name, function_id);
            let line = profile::Line {
                function_id,
                line: 0,
                ..Default::default()
            };
            let loc = profile::Location {
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
        let sample = profile::Sample {
            location_id: locs,
            value: vec![*cycles as i64, *cycles as i64 * 1_000_000_000 / frequency],
            label: vec![].into(),
            ..Default::default()
        };
        samples.push(sample);
    }
    let samples_value = profile::ValueType {
        field_type: strings[CYCLES] as i64,
        unit: strings[COUNT] as i64,
        ..Default::default()
    };
    let time_value = profile::ValueType {
        field_type: strings[CPU] as i64,
        unit: strings[NANOSECONDS] as i64,
        ..Default::default()
    };
    let profile = profile::Profile {
        sample_type: vec![samples_value, time_value.clone()].into(),
        sample: samples.into(),
        string_table: str_tbl.into(),
        function: fn_tbl.into(),
        location: loc_tbl.into(),
        period_type: Some(time_value).into(),
        period: 1_000_000_000 / frequency,
        ..Default::default()
    };
    let data = profile.write_to_bytes().expect("protobuf serialization");
    std::fs::write(&output_file, data)?;

    Ok(())
}

fn normalize_function_name(name: &str) -> String {
    name.replace("<", "{").replace(">", "}").to_string()
}
