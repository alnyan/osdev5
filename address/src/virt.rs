use super::PhysicalAddress;
use core::convert::TryFrom;
use core::fmt;
use core::iter::Step;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};

pub trait AddressSpace: Copy + Clone + PartialEq + PartialOrd {
    const NAME: &'static str;
    const OFFSET: usize;
    const LIMIT: usize;
}

pub trait NoTrivialConvert {}
pub trait TrivialConvert {}

#[repr(transparent)]
#[derive(Copy, Clone, PartialOrd, PartialEq)]
pub struct VirtualAddress<Kind: AddressSpace>(usize, PhantomData<Kind>);

// Arithmetic
impl<T: AddressSpace> Add<usize> for VirtualAddress<T> {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: usize) -> Self {
        // Will panic on overflow
        Self::from(self.0 + rhs)
    }
}
impl<T: AddressSpace> AddAssign<usize> for VirtualAddress<T> {
    #[inline(always)]
    fn add_assign(&mut self, rhs: usize) {
        // Will panic on overflow
        *self = Self::from(self.0 + rhs);
    }
}
impl<T: AddressSpace> Sub<usize> for VirtualAddress<T> {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: usize) -> Self {
        // Will panic on underflow
        Self::from(self.0 - rhs)
    }
}
impl<T: AddressSpace> SubAssign<usize> for VirtualAddress<T> {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: usize) {
        // Will panic on underflow
        *self = Self::from(self.0 - rhs);
    }
}

// Trivial conversion VirtualAddress -> PhysicalAddress
impl<T: AddressSpace + TrivialConvert> From<VirtualAddress<T>> for PhysicalAddress {
    #[inline(always)]
    fn from(virt: VirtualAddress<T>) -> Self {
        assert!(virt.0 < T::LIMIT);
        PhysicalAddress::from(virt.0 - T::OFFSET)
    }
}

// Formatting
impl<T: AddressSpace> fmt::Debug for VirtualAddress<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} {:#018x}>", T::NAME, self.0)
    }
}

impl<T: AddressSpace> VirtualAddress<T> {
    #[inline(always)]
    pub const fn null() -> Self {
        Self(0, PhantomData)
    }

    pub fn try_subtract(self, p: usize) -> Option<Self> {
        let (res, overflow) = self.0.overflowing_sub(p);
        if overflow || res < T::OFFSET || res >= T::LIMIT {
            None
        } else {
            Some(Self(res, PhantomData))
        }
    }

    #[inline]
    pub fn diff(start: Self, end: Self) -> isize {
        if end >= start {
            isize::try_from(end.0 - start.0).expect("Address subtraction overflowed")
        } else {
            -isize::try_from(start.0 - end.0).expect("Address subtraction overflowed")
        }
    }

    #[inline(always)]
    pub fn try_diff(start: Self, end: Self) -> Option<isize> {
        if end >= start {
            isize::try_from(end.0 - start.0).ok()
        } else {
            isize::try_from(start.0 - end.0).map(Neg::neg).ok()
        }
    }

    #[inline(always)]
    pub unsafe fn as_slice_mut<U>(self, count: usize) -> &'static mut [U] {
        core::slice::from_raw_parts_mut(self.0 as *mut _, count)
    }

    #[inline(always)]
    pub fn as_mut_ptr<U>(self) -> *mut U {
        self.0 as *mut U
    }

    #[inline(always)]
    pub fn as_ptr<U>(self) -> *const U {
        self.0 as *const U
    }

    #[inline(always)]
    pub unsafe fn as_mut<U>(self) -> Option<&'static mut U> {
        (self.0 as *mut U).as_mut()
    }

    #[inline(always)]
    pub unsafe fn from_ptr<U>(r: *const U) -> Self {
        Self::from(r as usize)
    }

    #[inline(always)]
    pub unsafe fn from_ref<U>(r: &U) -> Self {
        Self(r as *const U as usize, PhantomData)
    }
}

// Step
impl<T: AddressSpace> Step for VirtualAddress<T> {
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

// Conversion into VirtualAddress
impl<T: AddressSpace> const From<usize> for VirtualAddress<T> {
    #[inline(always)]
    fn from(p: usize) -> Self {
        if T::LIMIT > 0 {
            assert!(p >= T::OFFSET && p < T::LIMIT);
        }
        Self(p, PhantomData)
    }
}

#[cfg(target_pointer_width = "64")]
impl<T: AddressSpace> From<u64> for VirtualAddress<T> {
    #[inline(always)]
    fn from(p: u64) -> Self {
        Self::from(p as usize)
    }
}

// Conversion from VirtualAddress
impl<T: AddressSpace> From<VirtualAddress<T>> for usize {
    #[inline(always)]
    fn from(p: VirtualAddress<T>) -> Self {
        p.0
    }
}

#[cfg(target_pointer_width = "64")]
impl<T: AddressSpace> From<VirtualAddress<T>> for u64 {
    #[inline(always)]
    fn from(p: VirtualAddress<T>) -> Self {
        p.0 as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PhysicalAddress;

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
    fn test_trivial_construct_valid() {
        for i in 0x8000usize..0xC000 {
            VirtualAddress::<S0>::from(i);
        }
    }

    #[test]
    #[should_panic]
    fn test_trivial_construct_invalid_0() {
        let _v = VirtualAddress::<S0>::from(0x1234usize);
    }

    #[test]
    #[should_panic]
    fn test_trivial_construct_invalid_1() {
        let _v = VirtualAddress::<S0>::from(0xD123usize);
    }

    #[test]
    fn test_trivial_convert() {
        let v0 = VirtualAddress::<S0>::from(0x8123usize);
        assert_eq!(
            PhysicalAddress::from(v0),
            PhysicalAddress::from(0x123usize)
        );
    }

    #[test]
    fn test_add_valid() {
        let v0 = VirtualAddress::<S0>::from(0x8100usize);
        assert_eq!(VirtualAddress::<S0>::from(0x8223usize), v0 + 0x123usize);
    }

    #[test]
    #[should_panic]
    fn test_add_overflow() {
        let v0 = VirtualAddress::<S0>::from(0x8100usize);
        let _v = v0 - 0xF123usize;
    }

    #[test]
    fn test_subtract_valid() {
        let v0 = VirtualAddress::<S0>::from(0x8100usize);
        assert_eq!(VirtualAddress::<S0>::from(0x8023usize), v0 - 0xDDusize);
    }

    #[test]
    #[should_panic]
    fn test_subtract_overflow() {
        let v0 = VirtualAddress::<S0>::from(0x8100usize);
        let _v = v0 - 0x1234usize;
    }

    #[test]
    fn test_try_subtract() {
        let v0 = VirtualAddress::<S0>::from(0x8100usize);
        assert_eq!(v0.try_subtract(0x1234usize), None);
        assert_eq!(
            v0.try_subtract(0x12usize),
            Some(VirtualAddress::<S0>::from(0x80EEusize))
        );
    }

    #[test]
    fn test_add_assign_valid() {
        let mut v0 = VirtualAddress::<S0>::from(0x8100usize);
        v0 += 0x123usize;
        assert_eq!(v0, VirtualAddress::<S0>::from(0x8223usize));
    }

    #[test]
    fn test_sub_assign_valid() {
        let mut v0 = VirtualAddress::<S0>::from(0x8321usize);
        v0 -= 0x123usize;
        assert_eq!(v0, VirtualAddress::<S0>::from(0x81FEusize));
    }

    #[test]
    #[should_panic]
    fn test_sub_assign_overflow() {
        let mut v0 = VirtualAddress::<S0>::from(0x8321usize);
        v0 -= 0x1234usize;
    }

    #[test]
    #[should_panic]
    fn test_add_assign_overflow() {
        let mut v0 = VirtualAddress::<S0>::from(0x8321usize);
        v0 += 0xF234usize;
    }

    #[test]
    fn test_format() {
        let v0 = VirtualAddress::<S0>::from(0x8123usize);
        assert_eq!(&format!("{:?}", v0), "<S0 0x0000000000008123>");
    }

    #[test]
    fn test_diff() {
        let v0 = VirtualAddress::<S0>::from(0x8123usize);
        let v1 = VirtualAddress::<S0>::from(0x8321usize);

        // Ok
        assert_eq!(VirtualAddress::diff(v0, v1), 510);
        assert_eq!(VirtualAddress::diff(v1, v0), -510);
        assert_eq!(VirtualAddress::diff(v0, v0), 0);
        assert_eq!(VirtualAddress::diff(v1, v1), 0);
    }

    #[test]
    #[should_panic]
    fn test_diff_overflow() {
        let v0 = VirtualAddress::<S1>::from(0usize);
        let v1 = VirtualAddress::<S1>::from(usize::MAX);

        let _v = VirtualAddress::diff(v0, v1);
    }

    #[test]
    fn test_step() {
        let mut count = 0;
        for _ in VirtualAddress::<S0>::from(0x8000usize)..VirtualAddress::<S0>::from(0x8300usize) {
            count += 1;
        }
        assert_eq!(count, 0x300);

        let mut count = 0;
        for _ in (VirtualAddress::<S0>::from(0x8000usize)..VirtualAddress::<S0>::from(0x8300usize))
            .step_by(0x100)
        {
            count += 1;
        }
        assert_eq!(count, 3);

        let mut count = 0;
        for _ in
            (VirtualAddress::<S0>::from(0x8000usize)..VirtualAddress::<S0>::from(0x8300usize)).rev()
        {
            count += 1;
        }
        assert_eq!(count, 0x300);

        let mut count = 0;
        for _ in (VirtualAddress::<S0>::from(0x8000usize)..VirtualAddress::<S0>::from(0x8300usize))
            .rev()
            .step_by(0x100)
        {
            count += 1;
        }
        assert_eq!(count, 3);
    }
}
