///
/// Flag for argument info.
///
/// This represents the presence of special argument types in the method call.
///
/// ~~~~text
/// 0 0 0 0_0 0 1 1
///       | | | | |
///       | | | | +- 1: double splat hash args exists. 0: no keyword args,
///       | | | +--- 1: a block arg exists. 0: no block arg.
///       | | +----- 1: delegate args exist.
///       | +------- 1: hash splat args exist.
///       +--------- 1: splat args exists.
/// ~~~~
///
#[derive(Clone, Copy, PartialEq)]
pub struct ArgFlag(u8);

impl std::fmt::Debug for ArgFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {} {}",
            if self.has_block_arg() { "BLKARG" } else { "" },
            if self.has_hash_arg() { "HASH" } else { "" },
            if self.has_hash_splat() {
                "HASH_SPLAT"
            } else {
                ""
            },
            if self.has_delegate() { "DELEG" } else { "" },
            if self.has_splat() { "SPLAT" } else { "" },
        )
    }
}

impl ArgFlag {
    pub fn new(
        kw_flag: bool,
        block_flag: bool,
        delegate_flag: bool,
        hash_splat: bool,
        splat_flag: bool,
    ) -> Self {
        let f = (if kw_flag { 1 } else { 0 })
            + (if block_flag { 2 } else { 0 })
            + (if delegate_flag { 4 } else { 0 })
            + (if hash_splat { 8 } else { 0 })
            + (if splat_flag { 16 } else { 0 });
        Self(f)
    }

    pub fn default() -> Self {
        Self(0)
    }

    pub fn splat() -> Self {
        Self(16)
    }

    pub fn to_u8(self) -> u8 {
        self.0
    }

    pub fn from_u8(f: u8) -> Self {
        Self(f)
    }

    pub fn has_hash_arg(&self) -> bool {
        self.0 & 0b001 == 1
    }

    pub fn has_block_arg(&self) -> bool {
        self.0 & 0b010 == 2
    }

    pub fn has_delegate(&self) -> bool {
        self.0 & 0b100 == 4
    }

    pub fn has_hash_splat(&self) -> bool {
        self.0 & 0b1000 != 0
    }

    pub fn has_splat(&self) -> bool {
        self.0 & 0b1_0000 != 0
    }
}
