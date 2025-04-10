mod analyzer;
mod api;
#[cfg(target_family = "unix")]
pub mod arch_unix;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub mod arch_wasm;
#[cfg(all(target_family = "wasm", target_os = "wasi"))]
pub mod arch_wasm_wasi;
#[cfg(target_family = "windows")]
pub mod arch_windows;
mod machine_analyzer;
mod machine_assign;
mod machine_gdb;
mod misc;
mod syscall_elf_dumper;
mod syscall_file_operation;
mod syscall_file_stream;
mod syscall_file_write;
mod syscall_random;
mod syscall_stdio;
mod syscall_timestamp;

pub use analyzer::analyze;
pub use api::{run, run_json};
#[cfg(target_family = "unix")]
pub use arch_unix::{self as arch};
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use arch_wasm::{self as arch};
#[cfg(all(target_family = "wasm", target_os = "wasi"))]
pub use arch_wasm_wasi::{self as arch};
#[cfg(target_family = "windows")]
pub use arch_windows::{self as arch};
pub use machine_analyzer::{MachineAnalyzer, MachineOverlap, MachineProfile, MachineStepLog};
pub use machine_assign::MachineAssign;
pub use machine_gdb::{GdbStubHandler, GdbStubHandlerEventLoop};
pub use misc::{DummyResourceLoader, Embed, HumanReadableCycles, get_script_hash_by_index};
pub use syscall_elf_dumper::ElfDumper;
pub use syscall_file_operation::FileOperation;
pub use syscall_file_stream::FileStream;
pub use syscall_file_write::FileWriter;
pub use syscall_random::Random;
pub use syscall_stdio::Stdio;
pub use syscall_timestamp::Timestamp;
