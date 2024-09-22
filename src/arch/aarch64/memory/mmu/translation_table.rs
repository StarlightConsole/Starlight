use core::convert;

use tock_registers::{register_bitfields, registers::InMemoryRegister, interfaces::{Readable, Writeable}};

use crate::{bsp, memory::{self, mmu::{arch_mmu::{Granule512MiB, Granule64KiB}, AccessPermissions, AttributeFields, MemAttributes, MemoryRegion, PageAddress}, Address, Physical, Virtual}};

register_bitfields! {u64,
    STAGE1_TABLE_DESCRIPTOR [
        /// physical address of next descriptor
        NEXT_LEVEL_TABLE_ADDR_64KiB OFFSET(16) NUMBITS(32) [],

        TYPE OFFSET(1) NUMBITS(1) [
            Block = 0,
            Table = 1
        ],

        VALID OFFSET(0) NUMBITS(1) [
            False = 0,
            True = 1
        ]
    ]
}

register_bitfields! {u64,
    STAGE1_PAGE_DESCRIPTOR [
        /// unprivileged execute-never
        UXN OFFSET(54) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// privileged execute-never
        PXN OFFSET(53) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// physical address of next table descriptor (lvl2) or page descriptor (lvl3)
        OUTPUT_ADDR_64KiB OFFSET(16) NUMBITS(32) [],

        /// access flag
        AF OFFSET(10) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// shareability field
        SH OFFSET(8) NUMBITS(2) [
            OuterShareable = 0b10,
            InnerShareable = 0b11
        ],

        /// access permissions
        AP OFFSET(6) NUMBITS(2) [
            RW_EL1 = 0b00,
            RW_EL1_EL0 = 0b01,
            RO_EL1 = 0b10,
            RO_EL1_RL0 = 0b11
        ],

        /// memory attributes index into MAIR_EL1
        AttrIndx OFFSET(2) NUMBITS(3) [],

        TYPE OFFSET(1) NUMBITS(1) [
            Reserved_Invalid = 0,
            Page = 1
        ],

        VALID OFFSET(0) NUMBITS(1) [
            False = 0,
            True = 1
        ]
    ]
}

#[derive(Copy, Clone)]
#[repr(C)]
struct TableDescriptor {
    value: u64
}

#[derive(Copy, Clone)]
#[repr(C)]
struct PageDescriptor {
    value: u64
}

trait StartAddr {
    fn phys_start_addr(&self) -> Address<Physical>;
}

#[repr(C)]
#[repr(align(65536))]
pub struct FixedSizeTranslationTable<const NUM_TABLES: usize> {
    /// 64MiB windows
    lvl3: [[PageDescriptor; 8192]; NUM_TABLES],

    /// 512MiB windows
    lvl2: [TableDescriptor; NUM_TABLES],

    initialized: bool,
}

impl<T, const N: usize> StartAddr for [T; N] {
    fn phys_start_addr(&self) -> Address<Physical> {
        Address::new(self as *const _ as usize)
    }
}

impl TableDescriptor {
    pub const fn new_zeroed() -> Self {
        Self { value: 0 }
    }

    pub fn from_next_lvl_table_addr(phys_next_lvl_table_addr: Address<Physical>) -> Self {
        let val = InMemoryRegister::<u64, STAGE1_TABLE_DESCRIPTOR::Register>::new(0);

        let shifted = phys_next_lvl_table_addr.as_usize() >> Granule64KiB::SHIFT;
        val.write(
            STAGE1_TABLE_DESCRIPTOR::NEXT_LEVEL_TABLE_ADDR_64KiB.val(shifted as u64)
                  + STAGE1_TABLE_DESCRIPTOR::TYPE::Table
                  + STAGE1_TABLE_DESCRIPTOR::VALID::True
        );

        TableDescriptor { value: val.get() }
    }
}

impl convert::From<AttributeFields> for tock_registers::fields::FieldValue<u64, STAGE1_PAGE_DESCRIPTOR::Register> {
    fn from(attribute_fields: AttributeFields) -> Self {
        let mut desc = match attribute_fields.mem_attributes {
            MemAttributes::CacheableDRAM => {
                STAGE1_PAGE_DESCRIPTOR::SH::InnerShareable
                    + STAGE1_PAGE_DESCRIPTOR::AttrIndx.val(crate::memory::mmu::arch_mmu::mair::NORMAL)
            }
            MemAttributes::Device => {
                STAGE1_PAGE_DESCRIPTOR::SH::OuterShareable
                    + STAGE1_PAGE_DESCRIPTOR::AttrIndx.val(crate::memory::mmu::arch_mmu::mair::DEVICE)
            }
        };

        desc += match attribute_fields.access_permissions {
            AccessPermissions::ReadOnly => STAGE1_PAGE_DESCRIPTOR::AP::RO_EL1,
            AccessPermissions::ReadWrite => STAGE1_PAGE_DESCRIPTOR::AP::RW_EL1
        };

        desc += if attribute_fields.execute_never {
            STAGE1_PAGE_DESCRIPTOR::PXN::True
        } else {
            STAGE1_PAGE_DESCRIPTOR::PXN::False
        };

        // always set unprivileged execute-never as long as userspace is not yet implemented
        desc += STAGE1_PAGE_DESCRIPTOR::UXN::True;

        desc
    }
}

impl PageDescriptor {
    pub const fn new_zeroed() -> Self {
        Self { value: 0 }
    }

    pub fn from_output_addr(phys_output_addr: PageAddress<Physical>, attribute_fields: &AttributeFields) -> Self {
        let val = InMemoryRegister::<u64, STAGE1_PAGE_DESCRIPTOR::Register>::new(0);

        let shifted = phys_output_addr.into_inner().as_usize() >> Granule64KiB::SHIFT;
        val.write(
            STAGE1_PAGE_DESCRIPTOR::OUTPUT_ADDR_64KiB.val(shifted as u64)
                + STAGE1_PAGE_DESCRIPTOR::AF::True
                + STAGE1_PAGE_DESCRIPTOR::TYPE::Page
                + STAGE1_PAGE_DESCRIPTOR::VALID::True
                + (*attribute_fields).into()
        );

        Self { value: val.get() }
    }

    fn is_valid(&self) -> bool {
        InMemoryRegister::<u64, STAGE1_PAGE_DESCRIPTOR::Register>::new(self.value)
            .is_set(STAGE1_PAGE_DESCRIPTOR::VALID)
    }
}

impl<const AS_SIZE: usize> memory::mmu::AssociatedTranslationTable for memory::mmu::AddressSpace<AS_SIZE> where [u8; Self::SIZE >> Granule512MiB::SHIFT]: Sized {
    type TableStartFromBottom = FixedSizeTranslationTable<{ Self::SIZE >> Granule512MiB::SHIFT }>;
}

impl<const NUM_TABLES: usize> FixedSizeTranslationTable<NUM_TABLES> {
    pub const fn new() -> Self {
        assert!(bsp::memory::mmu::KernelGranule::SIZE == Granule64KiB::SIZE);
        assert!(NUM_TABLES > 0);

        Self {
            lvl3: [[PageDescriptor::new_zeroed(); 8192]; NUM_TABLES],
            lvl2: [TableDescriptor::new_zeroed(); NUM_TABLES],
            initialized: false,
        }
    }

    #[inline(always)]
    fn lvl2_lvl3_index_from_page_addr(&self, virt_page_addr: PageAddress<Virtual>) -> Result<(usize, usize), &'static str> {
        let addr = virt_page_addr.into_inner().as_usize();
        let lvl2_index = addr >> Granule512MiB::SHIFT;
        let lvl3_index = (addr & Granule512MiB::MASK) >> Granule64KiB::SHIFT;

        if lvl2_index > (NUM_TABLES - 1) {
            return Err("virtual page is out of bounds of translation table");
        }

        Ok((lvl2_index, lvl3_index))
    }

    #[inline(always)]
    fn set_page_descriptor_from_page_addr(&mut self, virt_page_addr: PageAddress<Virtual>, new_desc: &PageDescriptor) -> Result<(), &'static str> {
        let (lvl2_index, lvl3_index) = self.lvl2_lvl3_index_from_page_addr(virt_page_addr)?;
        let desc = &mut self.lvl3[lvl2_index][lvl3_index];

        if desc.is_valid() {
            return Err("virtal page is already mapped");
        }

        *desc = *new_desc;
        Ok(())
    }
}

impl<const NUM_TABLES: usize> memory::mmu::translation_table::interface::TranslationTable for FixedSizeTranslationTable<NUM_TABLES> {
    fn init(&mut self) {
        if self.initialized {
            return;
        }

        for (lvl2_nr, lvl2_entry) in self.lvl2.iter_mut().enumerate() {
            let phys_table_addr = self.lvl3[lvl2_nr].phys_start_addr();

            let new_desc = TableDescriptor::from_next_lvl_table_addr(phys_table_addr);
            *lvl2_entry = new_desc;
        }

        self.initialized = true;
    }

    fn phys_base_address(&self) -> Address<Physical> {
        self.lvl2.phys_start_addr()
    }

    unsafe fn map_at(&mut self, virt_region: &MemoryRegion<Virtual>, phys_region: &MemoryRegion<Physical>, attr: &AttributeFields) -> Result<(), &'static str> {
        assert!(self.initialized, "translation tables not initialized");

        if virt_region.size() != phys_region.size() {
            return Err("tried to map memory regions with unequal sizes");
        }

        if phys_region.end_exclusive_page_addr() > bsp::memory::phys_addr_space_end_exclusive_addr() {
            return Err("tried to map outside of physical address space");
        }

        let iter = phys_region.into_iter().zip(virt_region.into_iter());
        for (phys_page_addr, virt_page_addr) in iter {
            let new_desc = PageDescriptor::from_output_addr(phys_page_addr, attr);
            let virt_page = virt_page_addr;

            self.set_page_descriptor_from_page_addr(virt_page, &new_desc)?;
        }

        Ok(())
    }
}
