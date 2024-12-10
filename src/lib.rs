#![no_std]
#![allow(dead_code)]

pub(crate) mod regs;
pub(crate) mod descriptors;
pub(crate) mod transmit_head_wb;

use bitflags::bitflags;
use static_assertions::*;
use num_enum::TryFromPrimitive;

/*** Hardware Device Parameters of the Intel 82599 NIC (taken from the datasheet) ***/

/// The maximum number of receive descriptors per queue.
/// This is the maximum value that has been tested for the 82599 device.
pub(crate) const IXGBE_MAX_RX_DESC: u16            = 8192;
/// The maximum number of transmit descriptors per queue.
/// This is the maximum value that has been tested for the 82599 device.
pub(crate) const IXGBE_MAX_TX_DESC: u16            = 8192;
/// The maximum number of rx queues available on this NIC. 
pub(crate) const IXGBE_MAX_RX_QUEUES: u8           = 128;
/// The maximum number of tx queues available on this NIC.
pub(crate) const IXGBE_MAX_TX_QUEUES: u8           = 128;
/// The number of l34 5-tuple filters.
pub(crate) const NUM_L34_5_TUPLE_FILTERS: usize    = 128; 


/// Possible link speeds of the 82599 NIC
#[derive(PartialEq)]
pub enum LinkSpeedMbps {
    LS100 = 100,
    LS1000 = 1000,
    LS10000 = 10000, 
    LSUnknown = 0,
}

impl LinkSpeedMbps {
    /// Converts between a u32 and a LinkSpeedMbps enum.
    /// The u32 number is the value in the links register that represents the link speed.
    pub(crate) fn from_links_register_value(value: u32) -> LinkSpeedMbps {
        if value == (1 << 28) {
            Self::LS100
        } else if value == (2 << 28) {
            Self::LS1000
        } else if value == (3 << 28) {
            Self::LS10000
        } else {
            Self::LSUnknown
        }
    }
}

/// The set of receive buffer sizes that are accepted by the 82599 device.
#[derive(Copy, Clone)]
pub enum RxBufferSizeKiB {
    Buffer1KiB = 1,
    Buffer2KiB = 2,
    Buffer3KiB = 3,
    Buffer4KiB = 4,
    Buffer5KiB = 5,
    Buffer6KiB = 6,
    Buffer7KiB = 7,
    Buffer8KiB = 8,
    Buffer9KiB = 9,
    Buffer10KiB = 10,
    Buffer11KiB = 11,
    Buffer12KiB = 12,
    Buffer13KiB = 13,
    Buffer14KiB = 14,
    Buffer15KiB = 15,
    Buffer16KiB = 16
}


#[derive(Copy, Clone)]
pub enum NumDesc {
    Descs16 = 16,
    Descs512 = 512,
    Descs1k = 1024,
    Descs2k = 2048,
    Descs4k = 4096,
    Descs8k = 8192
}

bitflags! {
    /// A number that can take any value ranging in 7 bits
    pub struct U7: u8 {
        const B6      = 1 << 6;
        const B5      = 1 << 5;
        const B4      = 1 << 4;
        const B3      = 1 << 3;
        const B2      = 1 << 2;
        const B1      = 1 << 1;
        const B0      = 1 << 0;
    }
}


// Ensure that we never expose bit 7 as part of the `U7` interface.
const_assert_eq!(U7::all().bits() & 0x80, 0);

impl U7 {
    pub const fn zero() -> U7 {
        U7::from_bits_truncate(0)
    }
}

bitflags! {
    /// A number that can take any value ranging in 7 bits except 0
    pub struct HThresh: u8 {
        const B6      = 1 << 6;
        const B5      = 1 << 5;
        const B4      = 1 << 4;
        const B3      = 1 << 3;
        const B2      = 1 << 2;
        const B1      = 1 << 1;
        const B0      = 1 << 0;
    }
}

// Ensure that we never expose bit 7 as part of the `U7` interface.
const_assert_eq!(HThresh::all().bits() & 0x80, 0);

pub enum DescType {
    Legacy,
    AdvDesc1Buf,
    // AdvDescHeadSplit,
    // AdvDescHeadSplitAlways,
}


/***** For Filters *****/

/// The list of valid filter priority levels that can be used for the L5 filters. They range from (0,7).
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum L5FilterPriority {
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7
}

/// Options for the filter protocol used in the 5-tuple filters.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum L5FilterProtocol {
    Tcp = 0,
    Udp = 1,
    Sctp = 2,
    Other = 3
}

#[derive(Copy, Clone, Debug, TryFromPrimitive)]
#[repr(usize)]
pub enum L5FilterID {
    F0, F1, F2, F3, F4, F5, F6, F7, F8, F9, 
    F10, F11, F12, F13, F14, F15, F16, F17, F18, F19,
    F20, F21, F22, F23, F24, F25, F26, F27, F28, F29, 
    F30, F31, F32, F33, F34, F35, F36, F37, F38, F39, 
    F40, F41, F42, F43, F44, F45, F46, F47, F48, F49, 
    F50, F51, F52, F53, F54, F55, F56, F57, F58, F59, 
    F60, F61, F62, F63, F64, F65, F66, F67, F68, F69, 
    F70, F71, F72, F73, F74, F75, F76, F77, F78, F79, 
    F80, F81, F82, F83, F84, F85, F86, F87, F88, F89, 
    F90, F91, F92, F93, F94, F95, F96, F97, F98, F99, 
    F100, F101, F102, F103, F104, F105, F106, F107, F108, F109, 
    F110, F111, F112, F113, F114, F115, F116, F117, F118, F119, 
    F120, F121, F122, F123, F124, F125, F126, F127, 
}

bitflags! {
    // If a bit is set, then it is NOT used in the comparison
    pub struct L5FilterMaskFlags: u32 {
        const SOURCE_ADDRESS            = 1 << 25;
        const DESTINATION_ADDRESS       = 1 << 26;
        const SOURCE_PORT               = 1 << 27;
        const DESTINATION_PORT          = 1 << 28;
        const PROTOCOL                  = 1 << 29;
    }
}

impl L5FilterMaskFlags {
    pub const fn zero() -> L5FilterMaskFlags {
        L5FilterMaskFlags::from_bits_truncate(0)
    }
}

// Ensure that we never expose bits besides [29:25] as part of the `L5FilterMaskFlags` interface.
const_assert_eq!(L5FilterMaskFlags::all().bits() & 0xC1FF_FFFF, 0);



/***** For RSS *****/

#[derive(Copy, Clone, Debug, TryFromPrimitive)]
#[repr(usize)]
pub enum RedirectionTableReg {
    R0, R1, R2, R3, R4, R5, R6, R7, R8, R9, 
    R10, R11, R12, R13, R14, R15, R16, R17, R18, R19,
    R20, R21, R22, R23, R24, R25, R26, R27, R28, R29, 
    R30, R31
}

bitflags! {
    // The fields that should be used to calculate the hash
    pub struct RSSFieldFlags: u32 {
        const TCP_IPV4      = 1 << 16;
        const IPV4          = 1 << 17;
        const IPV6          = 1 << 20;
        const TCP_IPV6      = 1 << 21;
        const UDP_IPV4      = 1 << 22;
        const UDP_IPV6      = 1 << 23;
    }
}

// Ensure that we never expose bits besides [16:17] and [20:23] as part of the `RSSFieldFlags` interface.
const_assert_eq!(RSSFieldFlags::all().bits() & 0xFF0C_FFFF, 0);

#[cfg(not(prusti))]
impl RSSFieldFlags {
    pub const fn zero() -> RSSFieldFlags {
        RSSFieldFlags::from_bits_truncate(0)
    }
}

/// The list of valid Queues that can be used in the 82599 (0,64]
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, TryFromPrimitive)]
pub enum QueueID {
    Q0,Q1,Q2,Q3,Q4,Q5,Q6,Q7,Q8,Q9,
    Q10,Q11,Q12,Q13,Q14,Q15,Q16,Q17,Q18,Q19,
    Q20,Q21,Q22,Q23,Q24,Q25,Q26,Q27,Q28,Q29,
    Q30,Q31,Q32,Q33,Q34,Q35,Q36,Q37,Q38,Q39,
    Q40,Q41,Q42,Q43,Q44,Q45,Q46,Q47,Q48,Q49,
    Q50,Q51,Q52,Q53,Q54,Q55,Q56,Q57,Q58,Q59,
    Q60,Q61,Q62,Q63
}

impl From<RSSQueueID> for QueueID {
    fn from(qid: RSSQueueID) -> Self {
        match qid {
            RSSQueueID::Q0 => QueueID::Q0,
            RSSQueueID::Q1 => QueueID::Q1,
            RSSQueueID::Q2 => QueueID::Q2,
            RSSQueueID::Q3 => QueueID::Q3,
            RSSQueueID::Q4 => QueueID::Q4,
            RSSQueueID::Q5 => QueueID::Q5,
            RSSQueueID::Q6 => QueueID::Q6,
            RSSQueueID::Q7 => QueueID::Q7,
            RSSQueueID::Q8 => QueueID::Q8,
            RSSQueueID::Q9 => QueueID::Q9,
            RSSQueueID::Q10 => QueueID::Q10,
            RSSQueueID::Q11 => QueueID::Q11,
            RSSQueueID::Q12 => QueueID::Q12,
            RSSQueueID::Q13 => QueueID::Q13,
            RSSQueueID::Q14 => QueueID::Q14,
            RSSQueueID::Q15 => QueueID::Q15,
        }
    }
}

/// The list of valid Queues that can be used in the 82599 (0,64]
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, TryFromPrimitive)]
pub enum RSSQueueID {
    Q0,Q1,Q2,Q3,Q4,Q5,Q6,Q7,Q8,Q9,
    Q10,Q11,Q12,Q13,Q14,Q15
}

impl From<QueueID> for RSSQueueID {
    fn from(qid: QueueID) -> Self {
        match qid {
            QueueID::Q0 => RSSQueueID::Q0,
            QueueID::Q1 => RSSQueueID::Q1,
            QueueID::Q2 => RSSQueueID::Q2,
            QueueID::Q3 => RSSQueueID::Q3,
            QueueID::Q4 => RSSQueueID::Q4,
            QueueID::Q5 => RSSQueueID::Q5,
            QueueID::Q6 => RSSQueueID::Q6,
            QueueID::Q7 => RSSQueueID::Q7,
            QueueID::Q8 => RSSQueueID::Q8,
            QueueID::Q9 => RSSQueueID::Q9,
            QueueID::Q10 => RSSQueueID::Q10,
            QueueID::Q11 => RSSQueueID::Q11,
            QueueID::Q12 => RSSQueueID::Q12,
            QueueID::Q13 => RSSQueueID::Q13,
            QueueID::Q14 => RSSQueueID::Q14,
            QueueID::Q15 => RSSQueueID::Q15,
            _ => panic!("Invalid QueueID for RSSQueueID conversion")
        }
    }
}