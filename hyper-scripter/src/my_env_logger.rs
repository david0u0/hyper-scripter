use log::SetLoggerError;

pub fn try_init() -> Result<(), SetLoggerError> {
    #[cfg(feature = "log")]
    {
        env_logger::try_init()
    }
    #[cfg(feature = "no-log")]
    {
        Ok(())
    }
}

pub fn init() {
    #[cfg(feature = "log")]
    env_logger::init();
}
