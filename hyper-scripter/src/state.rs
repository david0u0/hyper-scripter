use std::cell::UnsafeCell;
#[cfg(not(feature = "no-state-check"))]
use std::sync::atomic::{AtomicU8, Ordering::SeqCst};

#[cfg(not(feature = "no-state-check"))]
mod consts {
    pub const UNINITIALIZED: u8 = 0;
    pub const INITIALIZING: u8 = 1;
    pub const INITIALIZED: u8 = 2;
}
#[cfg(not(feature = "no-state-check"))]
use consts::*;

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
            $state.set($f());
        });
    }};
}

#[macro_export]
macro_rules! local_global_state {
    ($mod_name:ident, $type_name:ident, $test_default:expr) => {
        mod $mod_name {
            use super::*;
            static GLOBAL: $crate::state::State<$type_name> = $crate::state::State::new();

            #[cfg(not(feature = "no-state-check"))]
            thread_local! {
                static LOCAL: std::cell::Cell<Option<&'static $type_name>> = std::cell::Cell::new(None);
            }

            pub fn get() -> &'static $type_name {
                #[cfg(test)]
                $crate::set_once!(GLOBAL, $test_default);

                #[cfg(not(feature = "no-state-check"))]
                if let Some(local) = LOCAL.get() {
                    return local;
                }

                GLOBAL.get()
            }

            pub fn set(data: $type_name) {
                #[cfg(test)]
                {
                let _ = data;
                log::info!("測試中，不設定狀態");
                }
                #[cfg(not(test))]
                GLOBAL.set(data);
            }

            #[allow(dead_code)]
            #[cfg(not(feature = "no-state-check"))]
            pub fn set_local(_data: &'static $type_name) {
                LOCAL.set(Some(_data));
            }
        }
    };
}

impl<T: Sized> State<T> {
    pub const fn new() -> State<T> {
        State {
            data: UnsafeCell::new(None),
            #[cfg(not(feature = "no-state-check"))]
            status: AtomicU8::new(UNINITIALIZED),
        }
    }

    pub fn set(&self, data: T) {
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
