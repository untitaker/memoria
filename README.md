# memento

A bad memory "profiler" for production.

1. Define a `UseCase` enum.
2. Use memento's custom allocator.
3. Assign usecases to functions or blocks of code using `Alloc::with_usecase`.
4. Get basic memory usage statistics per-usecase, either at the end of your
   program or periodically in a background thread.

[Documentation](https://docs.rs/memento), [Crates.io](https://crates.io/crates/memento)

<!-- Note: keep this codeblock in sync with examples/hello.rs -->

```rust
use num_enum::{TryFromPrimitive, IntoPrimitive};

#[derive(TryFromPrimitive, IntoPrimitive, Default, Debug)]
#[repr(u32)]
enum MyUseCase {
    #[default]
    None,
    LoadConfig,
    ProcessData,
}

impl memento::UseCase for MyUseCase {}

#[global_allocator]
static ALLOCATOR: memento::Alloc<MyUseCase> = memento::Alloc::new();

fn load_config() {
    let _guard = ALLOCATOR.with_usecase(MyUseCase::LoadConfig);
    println!("loading config...");
    // consume some memory
    let _temporary = vec![0u8; 256];
}

fn process_data() {
    let _guard = ALLOCATOR.with_usecase(MyUseCase::ProcessData);
    // consume some more memory
    let _temporary = vec![0u8; 2048];
}

fn main() {
    load_config();
    process_data();

    println!("memory usage stats:");
    ALLOCATOR.with_recorder(|recorder| {
        recorder.flush(
            |usecase, stat| {
                println!("{usecase:?}: {stat:?}");
            },
            |err, count| {
                println!("{err:?}: {count}");
            }
        );
        Ok(())
    }).ok();
}
```

## License

Licensed under the MIT license, see [`./LICENSE`](./LICENSE).
