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

const FLG_KW: u8 = 1;
const FLG_BLOCK: u8 = 2;
const FLG_DELEGATE: u8 = 4;
const FLG_HASH_SPLAT: u8 = 8;
const FLG_SPLAT: u8 = 16;

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
    #[inline(always)]
    pub fn new(
        kw_flag: bool,
        block_flag: bool,
        delegate_flag: bool,
        hash_splat: bool,
        splat_flag: bool,
    ) -> Self {
        let f = (if kw_flag { FLG_KW } else { 0 })
            + (if block_flag { FLG_BLOCK } else { 0 })
            + (if delegate_flag { FLG_DELEGATE } else { 0 })
            + (if hash_splat { FLG_HASH_SPLAT } else { 0 })
            + (if splat_flag { FLG_SPLAT } else { 0 });
        Self(f)
    }

    #[inline(always)]
    pub fn default() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub fn splat() -> Self {
        Self(16)
    }

    #[inline(always)]
    pub fn to_u8(self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub fn from_u8(f: u8) -> Self {
        Self(f)
    }

    #[inline(always)]
    pub fn has_hash_arg(&self) -> bool {
        self.0 & FLG_KW != 0
    }

    #[inline(always)]
    pub fn has_block_arg(&self) -> bool {
        self.0 & FLG_BLOCK != 0
    }

    #[inline(always)]
    pub fn has_delegate(&self) -> bool {
        self.0 & FLG_DELEGATE != 0
    }

    #[inline(always)]
    pub fn has_hash_splat(&self) -> bool {
        self.0 & FLG_HASH_SPLAT != 0
    }

    #[inline(always)]
    pub fn has_splat(&self) -> bool {
        self.0 & FLG_SPLAT != 0
    }
}
