// Vestigial, may not be used
#[derive(Debug)]
pub enum NmdFileTokenFormat {
    Address,
    Byte,
    Bytes(usize),
    Float,
    None,
    Short,
}
