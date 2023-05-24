use std::collections::BTreeMap;
use std::sync::Mutex;
use memoria::UseCase;

lazy_static::lazy_static! {
    static ref RESULTS: Mutex<BTreeMap<MyUseCase, usize>> = Mutex::new(BTreeMap::new());
}

memoria::usecase! {
    enum MyUseCase {
        default None,
        A,
        B,
        C,
        D,
        E,
        F,
    }

    impl memoria::UseCase for MyUseCase {
        fn on_alloc(&self, size: usize) {
            if matches!(self, MyUseCase::None) {
                return;
            }

            let _guard = Allocator::with_usecase(MyUseCase::None);
            if let Ok(mut map) = RESULTS.try_lock() {
                *map.entry(*self).or_insert(0) += size;
            }
        }

        fn on_dealloc(&self, size: usize) {
            if matches!(self, MyUseCase::None) {
                return;
            }

            let _guard = Allocator::with_usecase(MyUseCase::None);
            if let Ok(mut map) = RESULTS.try_lock() {
                let value = map.entry(*self).or_insert(0);
                // intentionally panic when value underflows
                assert!(*value >= size);
                *value -= size;
            }
        }
    }
}

type Allocator = memoria::Alloc<MyUseCase>;

#[global_allocator]
static ALLOCATOR: Allocator = memoria::new!();

type Allocations = BTreeMap<MyUseCase, Vec<String>>;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let mut allocations: Allocations = BTreeMap::new();

        fn test_allocate(allocations: &mut Allocations, usecase: MyUseCase) {
            let string = {
                let _guard = Allocator::with_usecase(usecase);
                "x".to_owned()
            };

            allocations.entry(usecase).or_insert_with(Vec::new).push(string);
        }

        fn test_deallocate(allocations: &mut Allocations, usecase: MyUseCase) {
            allocations.entry(usecase).or_insert_with(Vec::new).pop();
        }

        for &byte in data {
            if byte > 11 {
                return;
            }

            let usecase: MyUseCase = (byte as u32 / 2).into();
            if byte % 2 == 0 {
                test_allocate(&mut allocations, usecase);
            } else {
                test_deallocate(&mut allocations, usecase);
            }
        };

        for usecase in MyUseCase::all_variants() {
            if matches!(usecase, MyUseCase::None) {
                continue;
            }

            assert_eq!(
                allocations.entry(*usecase).or_insert_with(Vec::new).len(),
                RESULTS.lock().unwrap().get(usecase).cloned().unwrap_or_default()
            );
        }
    });
}
