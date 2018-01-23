use ::std::convert::From;
use ::std::mem;

const SIZE_LEN: usize = 4;
type SizeSlice = [u8; SIZE_LEN];

#[derive(Clone)]
pub struct Size(SizeSlice);

impl Size {
    pub fn bytes_no() -> usize {
        SIZE_LEN
    }

    pub fn zero() -> Self {
        Size([0; SIZE_LEN])        
    }

    pub fn slice(&self) -> &SizeSlice {
        &self.0
    }

    pub fn mut_slice(&mut self) -> &mut SizeSlice {
        &mut self.0
    }
}

impl From<u32> for Size {
    fn from(u: u32) -> Self {
        let mut s = Self::zero();
        s.0 = unsafe {
            mem::transmute::<u32, [u8; SIZE_LEN]>(u)
        };
        s
    }
}

impl From<usize> for Size {
    fn from(u: usize) -> Self {
        Self::from(u as u32)
    }
}

impl From<Size> for u32 {
    fn from(s: Size) -> Self {
        unsafe {
            mem::transmute::<[u8;  SIZE_LEN], u32>(s.0)
        }
    }
}

impl From<Size> for usize {
    fn from(s: Size) -> Self {
        let s: u32 = u32::from(s);
        s as Self
    }
}

// #[test]
// fn size_test() {
//     let max = u32::max_value();
//     for i in 0..max {
//         let s = Size::from(i);
//         assert_eq!(i, s.into());
//     }
// }
