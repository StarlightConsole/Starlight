use crate::{memory::{mmu as generic_mmu, mmu::*, Virtual, Physical}, synchronization::InitStateLock};

type KernelTranslationTable = <KernelVirtAddrSpace as AssociatedTranslationTable>::TableStartFromTop;

pub type KernelGranule = TranslationGranule<{ 64 * 1024 }>;
pub type KernelVirtAddrSpace = AddressSpace<{ kernel_virt_addr_space_size() }>;

#[link_section = ".data"]
#[no_mangle]
static KERNEL_TABLES: InitStateLock<KernelTranslationTable> = InitStateLock::new(KernelTranslationTable::new_for_precompute());

// this willbe patched to the correct value by the translation table tool after linking.
// the given value below is just a placeholder
#[link_section = ".text._start_arguments"]
#[no_mangle]
static PHYS_KERNEL_TABLES_BASE_ADDR: u64 = 0xC0FFEE33C0FFEE33;

const fn kernel_virt_addr_space_size() -> usize {
    let __kernel_virt_addr_space_size;

    include!("../kernel_virt_addr_space_size.ld");

    __kernel_virt_addr_space_size
}

const fn size_to_num_pages(size: usize) -> usize {
    assert!(size > 0);
    assert!(size % KernelGranule::SIZE == 0);

    size >> KernelGranule::SHIFT
}

fn virt_code_region() -> MemoryRegion<Virtual> {
    let num_pages = size_to_num_pages(super::code_size());

    let start_page_addr = super::virt_code_start();
    let end_exclusive_page_addr = start_page_addr.checked_offset(num_pages as isize).unwrap();

    MemoryRegion::new(start_page_addr, end_exclusive_page_addr)
}

fn virt_data_region() -> MemoryRegion<Virtual> {
    let num_pages = size_to_num_pages(super::data_size());

    let start_page_addr = super::virt_data_start();
    let end_exclusive_page_addr = start_page_addr.checked_offset(num_pages as isize).unwrap();

    MemoryRegion::new(start_page_addr, end_exclusive_page_addr)
}

pub fn virt_heap_region() -> MemoryRegion<Virtual> {
    let num_pages = size_to_num_pages(super::heap_size());

    let start_page_addr = super::virt_heap_start();
    let end_exclusive_page_addr = start_page_addr.checked_offset(num_pages as isize).unwrap();

    MemoryRegion::new(start_page_addr, end_exclusive_page_addr)
}

fn virt_boot_core_stack_region() -> MemoryRegion<Virtual> {
    let num_pages = size_to_num_pages(super::boot_core_stack_size());
    
    let start_page_addr = super::virt_boot_core_stack_start();
    let end_exclusive_page_addr = start_page_addr.checked_offset(num_pages as isize).unwrap();

    MemoryRegion::new(start_page_addr, end_exclusive_page_addr)
}

fn kernel_virt_to_phys_region(virt_region: MemoryRegion<Virtual>) -> MemoryRegion<Physical> {
    let phys_start_page_addr = generic_mmu::try_kernel_virt_page_addr_to_phys_page_addr(virt_region.start_page_addr()).unwrap();
    let phys_end_exclusive_page_addr = phys_start_page_addr.checked_offset(virt_region.num_pages() as isize).unwrap();

    MemoryRegion::new(phys_start_page_addr, phys_end_exclusive_page_addr)
}

fn kernel_page_attributes(virt_page_addr: PageAddress<Virtual>) -> AttributeFields {
    generic_mmu::try_kernel_page_attributes(virt_page_addr).unwrap()
}

pub fn kernel_translation_tables() -> &'static InitStateLock<KernelTranslationTable> {
    &KERNEL_TABLES
}

pub fn virt_mmio_remap_region() -> MemoryRegion<Virtual> {
    let num_pages = size_to_num_pages(super::mmio_remap_size());

    let start_page_addr = super::virt_mmio_remap_start();
    let end_exclusive_page_addr = start_page_addr.checked_offset(num_pages as isize).unwrap();

    MemoryRegion::new(start_page_addr, end_exclusive_page_addr)
}

pub fn kernel_add_mapping_records_for_precomputed() {
    let virt_code_region = virt_code_region();
    generic_mmu::kernel_add_mapping_record(
        "Kernel code and RO data",
        &virt_code_region,
        &kernel_virt_to_phys_region(virt_code_region),
        &kernel_page_attributes(virt_code_region.start_page_addr()),
    );

    let virt_data_region = virt_data_region();
    generic_mmu::kernel_add_mapping_record(
        "Kernel data and bss",
        &virt_data_region,
        &kernel_virt_to_phys_region(virt_data_region),
        &kernel_page_attributes(virt_data_region.start_page_addr()),
    );

    let virt_heap_region = virt_heap_region();
    generic_mmu::kernel_add_mapping_record(
        "Kernel heap",
        &virt_heap_region,
        &kernel_virt_to_phys_region(virt_heap_region),
        &kernel_page_attributes(virt_heap_region.start_page_addr()),
    );

    let virt_boot_core_stack_region = virt_boot_core_stack_region();
    generic_mmu::kernel_add_mapping_record(
        "Kernel boot-core stack",
        &virt_boot_core_stack_region,
        &kernel_virt_to_phys_region(virt_boot_core_stack_region),
        &kernel_page_attributes(virt_boot_core_stack_region.start_page_addr()),
    );
}
