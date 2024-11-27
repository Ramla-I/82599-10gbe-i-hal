use volatile::Volatile;
use zerocopy::FromBytes;

#[derive(FromBytes)]
#[repr(C)]
pub struct TransmitHead( Volatile<u32> );

impl TransmitHead {
    fn clear(&mut self) {
        self.0.write(0);
    }

    pub fn value(&self) -> u32 {
        self.0.read()
    }
}