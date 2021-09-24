//! Module for handling physical memory addresses

use crate::{AddressSpace, TrivialConvert, VirtualAddress};
use core::convert::TryFrom;
use core::fmt;
use core::iter::Step;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// Wrapper struct for representing a physical address
#[repr(transparent)]
#[derive(PartialEq, PartialOrd, Copy, Clone)]
pub struct PhysicalAddress(usize);

// Arithmetic
impl const Add<usize> for PhysicalAddress {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: usize) -> Self {
        // Will panic on overflow
        Self::from(self.0 + rhs)
    }
}
impl<A: Into<usize>> AddAssign<A> for PhysicalAddress {
    #[inline(always)]
    fn add_assign(&mut self, rhs: A) {
        // Will panic on overflow
        *self = Self::from(self.0 + rhs.into());
    }
}
impl const Sub<usize> for PhysicalAddress {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: usize) -> Self {
        Self::from(self.0 - rhs)
    }
}
impl SubAssign<usize> for PhysicalAddress {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: usize) {
        *self = Self::from(self.0 - rhs);
    }
}

// Construction
impl const From<usize> for PhysicalAddress {
    fn from(p: usize) -> Self {
        Self(p)
    }
}

#[cfg(target_pointer_width = "64")]
impl const From<u64> for PhysicalAddress {
    fn from(p: u64) -> Self {
        Self(p as usize)
    }
}

impl PhysicalAddress {
    /// Computes a signed difference between `start` and `end`
    /// addresses. Will panic if the result cannot be represented due to
    /// overflow.
    #[inline(always)]
    pub fn diff(start: PhysicalAddress, end: PhysicalAddress) -> isize {
        if end >= start {
            isize::try_from(end.0 - start.0).expect("Address subtraction overflowed")
        } else {
            -isize::try_from(start.0 - end.0).expect("Address subtraction overflowed")
        }
    }

    /// Computes a signed difference between `start` and `end`, does not check for
    /// overflow condition.
    ///
    /// # Safety
    ///
    /// Does not check for signed integer overflow.
    #[inline(always)]
    pub unsafe fn diff_unchecked(start: PhysicalAddress, end: PhysicalAddress) -> isize {
        end.0 as isize - start.0 as isize
    }

    /// Returns `true` if the address is page-aligned
    #[inline(always)]
    pub const fn is_paligned(self) -> bool {
        self.0 & 0xFFF == 0
    }

    /// Returns the index of the 4K page containing the address
    #[inline(always)]
    pub const fn page_index(self) -> usize {
        self.0 >> 12
    }
}

// Trivial conversion PhysicalAddress -> VirtualAddress
impl<T: AddressSpace + TrivialConvert> const From<PhysicalAddress> for VirtualAddress<T> {
    fn from(p: PhysicalAddress) -> Self {
        VirtualAddress::from(p.0 + T::OFFSET)
    }
}

impl const From<PhysicalAddress> for usize {
    #[inline(always)]
    fn from(p: PhysicalAddress) -> Self {
        p.0 as usize
    }
}

#[cfg(target_pointer_width = "64")]
impl const From<PhysicalAddress> for u64 {
    #[inline(always)]
    fn from(p: PhysicalAddress) -> Self {
        p.0 as u64
    }
}

// Formatting
impl fmt::Debug for PhysicalAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<phys {:#018x}>", self.0)
    }
}

// Step
impl Step for PhysicalAddress {
    #[inline]
    fn steps_between(_p0: &Self, _p1: &Self) -> Option<usize> {
        todo!()
    }

    #[inline]
    fn forward_checked(p: Self, steps: usize) -> Option<Self> {
        p.0.checked_add(steps).map(Self::from)
    }

    #[inline]
    fn backward_checked(p: Self, steps: usize) -> Option<Self> {
        p.0.checked_sub(steps).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AddressSpace, NoTrivialConvert, TrivialConvert, VirtualAddress};

    #[derive(Copy, Clone, PartialEq, PartialOrd)]
    struct S0;
    impl AddressSpace for S0 {
        const NAME: &'static str = "S0";
        const OFFSET: usize = 0x8000;
        const LIMIT: usize = Self::OFFSET + 0x4000;
    }
    impl TrivialConvert for S0 {}

    #[derive(Copy, Clone, PartialEq, PartialOrd)]
    struct S1;
    impl AddressSpace for S1 {
        const NAME: &'static str = "S1";
        const OFFSET: usize = 0;
        const LIMIT: usize = 0;
    }
    impl NoTrivialConvert for S1 {}

    #[test]
    fn test_virt_convert_valid() {
        let p0 = PhysicalAddress::from(0x1234usize);
        assert_eq!(
            VirtualAddress::<S0>::from(p0),
            VirtualAddress::<S0>::from(0x9234usize)
        );
    }

    #[test]
    #[should_panic]
    fn test_virt_convert_invalid() {
        let p0 = PhysicalAddress::from(0x4321usize);
        let _v = VirtualAddress::<S0>::from(p0);
    }
}
