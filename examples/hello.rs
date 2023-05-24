// Note: Keep in sync with README.md example
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(TryFromPrimitive, IntoPrimitive, Default, Debug)]
#[repr(u32)]
enum MyUseCase {
    #[default]
    None,
    LoadConfig,
    ProcessData,
}

impl memoria::UseCase for MyUseCase {}

#[global_allocator]
static ALLOCATOR: memoria::Alloc<MyUseCase> = memoria::Alloc::new();

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
    ALLOCATOR
        .with_recorder(|recorder| {
            recorder.flush(
                |usecase, stat| {
                    println!("{usecase:?}: {stat:?}");
                },
                |err, count| {
                    println!("{err:?}: {count}");
                },
            );
            Ok(())
        })
        .ok();
}
