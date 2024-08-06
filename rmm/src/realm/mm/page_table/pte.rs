pub mod shareable {
    pub const NON_SHAREABLE: u64 = 0b00;
    pub const RESERVED: u64 = 0b01;
    pub const OUTER: u64 = 0b10;
    pub const INNER: u64 = 0b11;
}

pub mod permission {
    pub const RW: u64 = 0b11;
    pub const WO: u64 = 0b10;
    pub const RO: u64 = 0b01;
    pub const NONE: u64 = 0b00;
}

// MemAttr[3] is not used in our configuration where FEAT_S2FWB (Forced Write Back) is set
// Refer to Table D8-04 Stage 2 memAttr[1:0] encoding 
//     when MemAttr[2] is 0, FEAT_S2FWB enabled in DDI0487K.a
// MemAttr[1:0]: if MemAttr[2] == 0
//      0b00 - Device-nGnRnE
//      0b01 - Device-nGnRE
//      0b10 - Device-nGRE
//      0b11 - Device-GRE
pub mod attribute {
    pub const NORMAL_FWB: u64 = 0b110;
    pub const NORMAL: u64 = 0b111;
    pub const NORMAL_NC: u64 = 0b101;
    pub const FWB_RESERVED: u64 = 0b100;
    pub const DEVICE_NGNRE: u64 = 0b001;
}

pub mod page_type {
    pub const BLOCK: u64 = 0b0;
    pub const TABLE_OR_PAGE: u64 = 0b1;
}
