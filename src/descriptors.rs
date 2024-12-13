use volatile::Volatile;
use bit_field::BitField;
use crate::regs::ReportStatusBit;

/// Advanced Transmit Descriptor used by the `ixgbe` NIC driver.
///
/// # Two usage modes
/// It has 2 modes: Read and Write Back, both of which use the whole 128 bits. 
/// There is one transmit descriptor per packet buffer; it can be converted between these 2 modes.
///
/// Read contains the addresses that the driver writes.
/// Write Back contains information the hardware writes on receiving a packet.
///
/// More information can be found in the 82599 datasheet.
#[repr(C)]
pub struct AdvancedTxDescriptor {
    /// Physical address of the packet buffer.
    pub buffer_addr:  Volatile<u64>,
    /// Contains the subfields:
    /// * data length:          [15:0]
    /// * mac:                  [19:18]
    /// * dtyp: Descriptor Type [23:20]
    /// * dcmd:                 [31:24]
    /// * status:               [35:32]
    /// * popts: options to offload checksum calculation [45:40]
    /// * paylen: the size in bytes of the data buffer in host memory, 
    /// not including the fields that the hardware adds [63:46]
    pub metadata:  Volatile<u64>,
}

impl AdvancedTxDescriptor {
    #[inline(always)]
    pub(crate) fn send(&mut self, buffer_addr: usize, buffer_length_in_bytes: u16, rs_bit: ReportStatusBit) {
        const TX_PAYLEN_SHIFT:                  u64 = 46;
        const TX_DCMD_RS_SHIFT:                 u64 = 24 + 3;     
        const TX_DCMD_EOP:                      u64 = 1 << 24;     
        const TX_DCMD_IFCS:                     u64 = 1 << (24 + 1);     
        const TX_DCMD_DEXT:                     u64 = 1 << (24 + 5);  
        const TX_DTYP_ADV:                      u64 = 0x3 << 20;

        let rs_bit: u64 = rs_bit.value() << TX_DCMD_RS_SHIFT; //if rs_set { TX_DCMD_RS } else { 0 };

        self.buffer_addr.write(buffer_addr as u64);
        self.metadata.write(
            ((buffer_length_in_bytes as u64) << TX_PAYLEN_SHIFT) | 
            (TX_DCMD_DEXT | TX_DCMD_IFCS | TX_DCMD_EOP | rs_bit) |
            TX_DTYP_ADV | // Advanced data descriptor
            buffer_length_in_bytes as u64 // length in bytes of data buffer
        );
    }

    #[inline(always)]
    pub(crate) fn packet_address(&self) -> u64 {
        self.buffer_addr.read()
    }

    pub fn descriptor_done(&self) -> bool {
        const TX_STATUS_DD: u64 = 1 << 32;

        let metadata = self.metadata.read();
        (metadata & TX_STATUS_DD) == TX_STATUS_DD
    }

    pub fn clear(&mut self) {
        self.buffer_addr.write(0);
        self.metadata.write(0);
    }
}


/// Advanced Receive Descriptor used to convey packet information to the device.
/// It has 2 modes: Read and Write Back, both of which use the whole 128 bits. 
/// Read contains the addresses that the driver writes.
/// Write Back contains information the hardware writes on receiving a packet.
#[repr(C)]
pub struct AdvancedRxDescriptor {
    /// Starting physcal address of the packet buffer.
    pub packet_buffer_addr:  Volatile<u64>,
    /// Starting physcal address of the header buffer.
    /// This field will only be used if header splitting is enabled. 
    pub header_buffer_addr:  Volatile<u64>,
}

impl AdvancedRxDescriptor {
    #[inline(always)]
    /// Descriptor Read mode
    pub(crate) fn set_packet_address(&mut self, packet_buffer_address: usize) {
        self.packet_buffer_addr.write(packet_buffer_address as u64);
        self.header_buffer_addr.write(0);
    }

    #[inline(always)]
    /// Descriptor Read mode
    pub(crate) fn packet_address(&self) -> u64 {
        self.packet_buffer_addr.read()
    }

    #[inline(always)]
    /// Write Back mode function for the Advanced Receive Descriptor.
    /// Returns the tuple: 
    /// * descriptor done (bit 0) 
    /// * packet length (bits [47:32])
    pub fn rx_metadata(&self) -> (bool, u16) {
        pub const RX_STATUS_DD: u64 = 1 << 0;
        pub const RX_PKT_LEN_SHIFT: u64 = 32;

        let metadata = self.header_buffer_addr.read();
        ((metadata & RX_STATUS_DD as u64) == RX_STATUS_DD as u64, (metadata >> RX_PKT_LEN_SHIFT) as u16 & 0xFFFF)
    }

    /// Write Back mode function for the Advanced Receive Descriptor.
    /// Returns the packet type that was used for the Receive Side Scaling hash function.
    pub fn get_rss_type(&self) -> u64{
        self.packet_buffer_addr.read().get_bits(0..3) 
    }

    /// Write Back mode function for the Advanced Receive Descriptor.
    /// Returns the packet type as identified by the hardware.
    pub fn get_packet_type(&self) -> u64{
        self.packet_buffer_addr.read().get_bits(4..16) 
    }

    /// Write Back mode function for the Advanced Receive Descriptor.
    /// Returns the Receive Side Scaling hash.
    pub fn get_rss_hash(&self) -> u64{
        self.packet_buffer_addr.read().get_bits(32..63) 
    }

    /// Write Back mode function for the Advanced Receive Descriptor.
    /// Returns the Flow Director Filter ID if the packet matches a filter.
    pub fn get_fdf_id(&self) -> u64{
        self.packet_buffer_addr.read().get_bits(32..63) 
    }
    
    /// Write Back mode function for the Advanced Receive Descriptor.
    /// Returns errors reported by hardware for different packet types
    pub fn get_ext_error(&self) -> u64{
        self.header_buffer_addr.read().get_bits(20..31) 
    }
}


/********* LEGACY DESCRIPTORS *********/
// TO DO: should eventually delete?

/// Descriptor used to share information about packet between device and driver. 
/// It's the descriptor type used in older Intel NICs and the E1000 driver.
#[repr(C)]
pub struct LegacyTxDescriptor {
    /// The physical address of the packet buffer
    pub buffer_addr:  Volatile<u64>,
    /// Packet metadata:
    /// * length    [15:0]
    /// * CSO       [23:16]
    /// * CMD       [31:24]
    /// * STATUS    [35:32]
    /// * CSS       [47: 40]
    /// * VLAN tag  [63: 48]
    pub metadata: Volatile<u64>
}

impl LegacyTxDescriptor {
    fn init(&mut self) {
        self.buffer_addr.write(0);
        self.metadata.write(0);
    }

    #[inline(always)]
    pub fn send(&mut self, transmit_buffer_addr: usize, transmit_buffer_length: u16, rs_set: bool) {
        const TX_CMD_EOP: u64 = 1 << 24;
        const TX_CMD_IFCS: u64 = 1 << (24 + 1);
        let rs_bit: u64 = if rs_set { 1 << (24 + 3) } else { 0 };

        self.buffer_addr.write(transmit_buffer_addr as u64);
        self.metadata.write(transmit_buffer_length as u64 | TX_CMD_EOP | TX_CMD_IFCS | rs_bit);
    }

    #[inline(always)]
    pub fn desc_done(&self) -> bool {
        const TX_STATUS_DD: u64 = 1 << 32;
        (self.metadata.read() & TX_STATUS_DD) == TX_STATUS_DD
    }
}

impl core::fmt::Debug for LegacyTxDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "{{Legacy TX DESC: addr: {:#X} metadata: {:#X}}}",
                    self.buffer_addr.read(), self.metadata.read())
    }
}

/// Legacy Receive Descriptor used to share packet information with the device. 
/// The driver writes to the upper 64 bits, and the NIC writes to the lower 64 bits.
/// It's the descriptor type used in older Intel NICs and the E1000 driver.
#[repr(C)]
pub struct LegacyRxDescriptor {
    /// The physical address of the receive buffer
    pub buffer_addr:  Volatile<u64>,   
    /// Packet metadata:
    /// * length            [15:0]
    /// * fragment checksum [31: 16]
    /// * status            [39: 32]
    /// * errors            [47: 40]
    /// * VLAN tag          [63: 48]
    pub metadata: Volatile<u64>,
}

impl LegacyRxDescriptor {
    pub fn init(&mut self, packet_buffer_address: usize) {
        self.buffer_addr.write(packet_buffer_address as u64);
        self.metadata.write(0);
    }

    #[inline(always)]
    pub fn set_packet_address(&mut self, packet_buffer_address: usize) {
        self.buffer_addr.write(packet_buffer_address as u64);
        self.metadata.write(0);
    }

    /// Returns the tuple:
    /// * descriptor done (bit 32) 
    /// * packet length (bits 15:0)
    #[inline(always)]
    pub fn rx_metadata(&self) -> (bool, u16) {
        const RX_METADATA_DD: u64 = 1 << 32;

        let metadata = self.metadata.read();
        ((metadata & (RX_METADATA_DD)) == (RX_METADATA_DD), metadata as u16 & 0xFFFF)
    }

    pub fn reset_status(&mut self) {
        self.metadata.write(0);
    }

    pub fn clear(&mut self) {
        self.buffer_addr.write(0);
        self.metadata.write(0);
    }
}

impl core::fmt::Debug for LegacyRxDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "{{Legacy RX DESC: addr: {:#X} metadata: {:#X}}}",
                    self.buffer_addr.read(), self.metadata.read())
    }
}