macro_rules! dcs_basic_command {
    (
        #[doc = $tt:tt]
        $instr_name:ident,
        $instr:expr
    ) => {
        #[doc = $tt]
        pub struct $instr_name;

        impl DcsCommand for $instr_name {
            fn instruction(&self) -> u8 {
                $instr
            }

            fn fill_params_buf(&self, _buffer: &mut [u8]) -> usize {
                0
            }
        }
    };
}

// Re-export the macro so that it can be accessed from outside this module via
// `crate::dcs::macros::dcs_basic_command!` without needing a duplicate `mod`.
pub(crate) use dcs_basic_command;
