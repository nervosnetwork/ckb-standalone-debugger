# File write

File write is a new cross-platform file writing syscall.

```c
int file_write(const char* path, void *ptr, size_t size)
```

- In windows, *nix, wasm-wasi, it will write the content to the local file system;
- In the wasm browser environment, it will write the data in hexadecimal format to localstorage.


## Usage

**C**

```c
char *data = "Hello World!\n";
size_t size = 13;
syscall(9013, "file_write_foo.txt", data, size, 0, 0, 0);
```

**Rust**

```rs
syscall(
    CString::from_str("file_write_foo.txt").unwrap().as_ptr() as u64,
    "Hello World!\n".as_bytes().as_ptr() as u64,
    13,
    0,
    0,
    0,
    0,
    9013,
);
```
