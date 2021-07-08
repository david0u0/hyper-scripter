use std::cell::UnsafeCell;
#[cfg(not(feature = "no-state-check"))]
use std::sync::atomic::{AtomicU8, Ordering::SeqCst};

const UNINITIALIZED: u8 = 0;
const INITIALIZING: u8 = 1;
const INITIALIZED: u8 = 2;

pub struct State<T> {
    data: UnsafeCell<Option<T>>,
    #[cfg(not(feature = "no-state-check"))]
    status: AtomicU8,
}
unsafe impl<T> Sync for State<T> {}

#[macro_export]
macro_rules! set_once {
    ($state:expr, $f:expr) => {{
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            #[cfg(test)]
            $state.set_test($f());
            #[cfg(not(test))]
            $state.set($f());
        });
    }};
}

impl<T: Sized> State<T> {
    pub const fn new() -> State<T> {
        State {
            data: UnsafeCell::new(None),
            #[cfg(not(feature = "no-state-check"))]
            status: AtomicU8::new(UNINITIALIZED),
        }
    }

    #[cfg(test)]
    pub fn set(&self, _data: T) {}
    #[cfg(test)]
    pub fn set_test(&self, data: T) {
        self.set_inner(data)
    }
    #[cfg(not(test))]
    pub fn set(&self, data: T) {
        self.set_inner(data)
    }

    fn set_inner(&self, data: T) {
        #[cfg(not(feature = "no-state-check"))]
        {
            let status = self
                .status
                .compare_exchange(UNINITIALIZED, INITIALIZING, SeqCst, SeqCst);
            log::debug!("設定前的狀態：{:?}", status);
            if status.is_err() {
                panic!("多次設定狀態");
            }
        }
        let ptr = self.data.get();
        unsafe {
            *ptr = Some(data);
        }
        #[cfg(not(feature = "no-state-check"))]
        self.status.store(INITIALIZED, SeqCst);
    }
    pub fn get(&self) -> &T {
        #[cfg(not(feature = "no-state-check"))]
        match self.status.load(SeqCst) {
            UNINITIALIZED => {
                panic!("還沒設定就取狀態");
            }
            INITIALIZING => {
                while self.status.load(SeqCst) == INITIALIZING {
                    std::hint::spin_loop();
                }
            }
            _ => (),
        }
        let ptr = self.data.get();
        unsafe { (&*ptr).as_ref().unwrap() }
    }
}
