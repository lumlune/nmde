use {
    crate::io::nmd::anatomy::token::NmdFileTokenFormat,
};

#[derive(Debug)]
pub struct NmdFileTokenValue {
    pub format: NmdFileTokenFormat,
    pub is_relative: bool,
    pub offset: usize,
}

// Can't use `Default` in a const context, so workaround
impl NmdFileTokenValue {
    pub const DEFAULT: Self = Self {
        // Format field is vestigial, but keep for debugging
        format: NmdFileTokenFormat::None,
        is_relative: false,
        offset: 0x0000,
    };
}

