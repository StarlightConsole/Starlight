use core::{iter::Step, num::NonZeroUsize, ops::Range};

use crate::{bsp, common, memory::{Address, AddressType, Physical}};

#[derive(Copy, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct PageAddress<ATYPE: AddressType> {
    inner: Address<ATYPE>,
}

#[derive(Copy, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct MemoryRegion<ATYPE: AddressType> {
    start: PageAddress<ATYPE>,
    end_exclusive: PageAddress<ATYPE>,
}

#[derive(Copy, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub enum MemAttributes {
    CacheableDRAM,
    Device,
}

#[derive(Copy, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub enum AccessPermissions {
    ReadOnly,
    ReadWrite,
}

#[derive(Copy, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct AttributeFields {
    pub mem_attributes: MemAttributes,
    pub access_permissions: AccessPermissions,
    pub execute_never: bool,
}

#[derive(Copy, Clone)]
pub struct MMIODescriptor {
    start_addr: Address<Physical>,
    end_addr_exclusive: Address<Physical>,
}

impl<ATYPE: AddressType> PageAddress<ATYPE> {
    #[allow(unused)]
    pub const MAX: Self = PageAddress {
        inner: Address::new(usize::MAX).align_down_page(),
    };

    pub fn into_inner(self) -> Address<ATYPE> {
        self.inner
    }

    pub fn checked_offset(self, count: isize) -> Option<Self> {
        if count == 0 {
            return Some(self);
        }

        let delta = count.unsigned_abs().checked_mul(bsp::memory::mmu::KernelGranule::SIZE)?;
        let result = if count.is_positive() {
            self.inner.as_usize().checked_add(delta)?
        } else {
            self.inner.as_usize().checked_sub(delta)?
        };

        Some(Self {
            inner: Address::new(result),
        })
    }
}

impl<ATYPE: AddressType> From<usize> for PageAddress<ATYPE> {
    fn from(addr: usize) -> Self {
        assert!(common::is_aligned(addr, bsp::memory::mmu::KernelGranule::SIZE), "input size not page aligned");

        Self {
            inner: Address::new(addr),
        }
    }
}

impl<ATYPE: AddressType> From<Address<ATYPE>> for PageAddress<ATYPE> {
    fn from(addr: Address<ATYPE>) -> Self {
        assert!(addr.is_page_aligned(), "input address not page aligned");

        Self {
            inner: addr,
        }
    }
}

impl<ATYPE: AddressType> Step for PageAddress<ATYPE> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if start > end {
            return None;
        }

        Some((end.inner.as_usize() - start.inner.as_usize()) >> bsp::memory::mmu::KernelGranule::SHIFT)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        start.checked_offset(count as isize)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        start.checked_offset(-(count as isize))
    }
}

impl<ATYPE: AddressType> MemoryRegion<ATYPE> {
    pub fn new(start: PageAddress<ATYPE>, end_exclusive: PageAddress<ATYPE>) -> Self {
        assert!(start <= end_exclusive);

        Self {
            start,
            end_exclusive,
        }
    }

    fn as_range(&self) -> Range<PageAddress<ATYPE>> {
        self.into_iter()
    }

    pub fn start_page_addr(&self) -> PageAddress<ATYPE> {
        self.start
    }

    pub fn start_addr(&self) -> Address<ATYPE> {
        self.start.into_inner()
    }

    pub fn end_exclusive_page_addr(&self) -> PageAddress<ATYPE> {
        self.end_exclusive
    }

    pub fn end_inclusive_page_addr(&self) -> PageAddress<ATYPE> {
        self.end_exclusive.checked_offset(-1).unwrap()
    }

    #[allow(unused)]
    pub fn contains(&self, addr: Address<ATYPE>) -> bool {
        let page_addr = PageAddress::from(addr.align_down_page());
        self.as_range().contains(&page_addr)
    }

    #[allow(unused)]
    pub fn overlaps(&self, other_region: &Self) -> bool {
        let self_range = self.as_range();

        self_range.contains(&other_region.start_page_addr())
            || self_range.contains(&other_region.end_inclusive_page_addr())
    }

    pub fn num_pages(&self) -> usize {
        PageAddress::steps_between(&self.start, &self.end_exclusive).unwrap()
    }

    pub fn size(&self) -> usize {
        let end_exclusive = self.end_exclusive.into_inner().as_usize();
        let start = self.start.into_inner().as_usize();

        end_exclusive - start
    }

    pub fn take_first_n_pages(&mut self, num_pages: NonZeroUsize) -> Result<Self, &'static str> {
        let count: usize = num_pages.into();

        let left_end_exclusive = self.start.checked_offset(count as isize);
        let left_end_exclusive = match left_end_exclusive {
            None => return Err("overflow while calculating left_end_exclusive"),
            Some(x) => x,
        };

        if left_end_exclusive > self.end_exclusive {
            return Err("not enough pages free pages");
        }

        let allocation = Self {
            start: self.start,
            end_exclusive: left_end_exclusive,
        };

        self.start = left_end_exclusive;

        Ok(allocation)
    }
}

impl<ATYPE: AddressType> IntoIterator for MemoryRegion<ATYPE> {
    type Item = PageAddress<ATYPE>;
    type IntoIter = Range<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        Range {
            start: self.start,
            end: self.end_exclusive,
        }
    }
}

impl From<MMIODescriptor> for MemoryRegion<Physical> {
    fn from(desc: MMIODescriptor) -> Self {
        let start = PageAddress::from(desc.start_addr.align_down_page());
        let end_exclusive = PageAddress::from(desc.end_addr_exclusive().align_up_page());

        Self {
            start,
            end_exclusive,
        }
    }
}

impl MMIODescriptor {
    pub const fn new(start_addr: Address<Physical>, size: usize) -> Self {
        assert!(size > 0);
        let end_addr_exclusive = Address::new(start_addr.as_usize() + size);

        Self {
            start_addr,
            end_addr_exclusive,
        }
    }

    pub const fn start_addr(&self) -> Address<Physical> {
        self.start_addr
    }

    // WARN: maybe not const?
    pub const fn end_addr_exclusive(&self) -> Address<Physical> {
        self.end_addr_exclusive
    }
}
