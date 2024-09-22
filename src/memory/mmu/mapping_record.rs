use crate::{bsp, common, info, memory::{mmu::AccessPermissions, Address, Physical, Virtual}, synchronization::{interface::ReadWriteEx, InitStateLock}, warn};

use super::{AttributeFields, MMIODescriptor, MemAttributes, MemoryRegion};

#[derive(Copy, Clone)]
struct MappingRecordEntry {
    pub users: [Option<&'static str>; 5],
    pub phys_start_addr: Address<Physical>,
    pub virt_start_addr: Address<Virtual>,
    pub num_pages: usize,
    pub attribute_fields: AttributeFields,
}

struct MappingRecord {
    inner: [Option<MappingRecordEntry>; 12],
}

static KERNEL_MAPPING_RECORD: InitStateLock<MappingRecord> = InitStateLock::new(MappingRecord::new());

impl MappingRecordEntry {
    pub fn new(name: &'static str, virt_region: &MemoryRegion<Virtual>, phys_region: &MemoryRegion<Physical>, attr: &AttributeFields) -> Self {
        Self {
            users: [Some(name), None, None, None, None],
            phys_start_addr: phys_region.start_addr(),
            virt_start_addr: virt_region.start_addr(),
            num_pages: phys_region.num_pages(),
            attribute_fields: *attr,
        }
    }

    fn find_next_free_user(&mut self) -> Result<&mut Option<&'static str>, &'static str> {
        if let Some(x) = self.users.iter_mut().find(|x| x.is_none()) {
            return Ok(x);
        }

        Err("storage for user info exhausted")
    }

    pub fn add_user(&mut self, user: &'static str) -> Result<(), &'static str> {
        let x = self.find_next_free_user()?;
        *x = Some(user);
        Ok(())
    }
}

impl MappingRecord {
    pub const fn new() -> Self {
        Self {
            inner: [None; 12]
        }
    }

    fn size(&self) -> usize {
        self.inner.iter().filter(|x| x.is_some()).count()
    }

    fn sort(&mut self) {
        let upper_bound_exclusive = self.size();
        let entries = &mut self.inner[0..upper_bound_exclusive];

        if !entries.is_sorted_by_key(|item| item.unwrap().virt_start_addr) {
            entries.sort_unstable_by_key(|item| item.unwrap().virt_start_addr)
        }
    }

    fn find_next_free(&mut self) -> Result<&mut Option<MappingRecordEntry>, &'static str> {
        if let Some(x) = self.inner.iter_mut().find(|x| x.is_none()) {
            return Ok(x);
        }

        Err("storage for user info exhausted")
    }

    fn find_duplicate(&mut self, phys_region: &MemoryRegion<Physical>) -> Option<&mut MappingRecordEntry> {
        self.inner
            .iter_mut()
            .filter_map(|x| x.as_mut())
            .filter(|x| x.attribute_fields.mem_attributes == MemAttributes::Device)
            .find(|x| {
                if x.phys_start_addr != phys_region.start_addr() {
                    return false;
                }

                if x.num_pages != phys_region.num_pages() {
                    return false;
                }

                true
            })
    }

    pub fn add(&mut self, name: &'static str, virt_region: &MemoryRegion<Virtual>, phys_region: &MemoryRegion<Physical>, attr: &AttributeFields) -> Result<(), &'static str> {
        let x = self.find_next_free()?;

        *x = Some(MappingRecordEntry::new(name, virt_region, phys_region, attr));

        self.sort();

        Ok(())
    }

    pub fn print(&self) {
        info!("      -------------------------------------------------------------------------------------------------------------------------------------------");
        info!("      {:^44}     {:^30}   {:^7}   {:^9}   {:^35}", "Virtual", "Physical", "Size", "Attr", "Entity");
        info!("      -------------------------------------------------------------------------------------------------------------------------------------------");

        for i in self.inner.iter().flatten() {
            let size = i.num_pages * bsp::memory::mmu::KernelGranule::SIZE;

            let virt_start = i.virt_start_addr;
            let virt_end_inclusive = virt_start + (size - 1);

            let phys_start = i.phys_start_addr;
            let phys_end_inclusive = phys_start + (size - 1);

            let (size, unit) = common::size_human_readable_ceil(size);

            let attr = match i.attribute_fields.mem_attributes {
                MemAttributes::CacheableDRAM => "C",
                MemAttributes::Device => "Dev",
            };

            let acc_p = match i.attribute_fields.access_permissions {
                AccessPermissions::ReadOnly => "RO",
                AccessPermissions::ReadWrite => "RW",
            };

            let xn = if i.attribute_fields.execute_never {
                "XN"
            } else {
                "X"
            };

            info!("      {}..{} --> {}..{} | {:>3} {} | {:<3} {} {:<2} | {}", virt_start, virt_end_inclusive, phys_start, phys_end_inclusive, size, unit, attr, acc_p, xn, i.users[0].unwrap());

            for k in i.users[1..].iter() {
                if let Some(additional_user) = *k {
                    info!("                                                                                                            | {}", additional_user);
                }
            }

            info!("      -------------------------------------------------------------------------------------------------------------------------------------------");
        }
    }
}

pub fn kernel_add(name: &'static str, virt_region: &MemoryRegion<Virtual>, phys_region: &MemoryRegion<Physical>, attr: &AttributeFields) -> Result<(), &'static str> {
    KERNEL_MAPPING_RECORD.write(|mr| mr.add(name, virt_region, phys_region, attr))
}

pub fn kernel_find_and_insert_mmio_duplicate(mmio_descriptor: &MMIODescriptor, new_user: &'static str) -> Option<Address<Virtual>> {
    let phys_region: MemoryRegion<Physical> = (*mmio_descriptor).into();

    KERNEL_MAPPING_RECORD.write(|mr| {
        let dup = mr.find_duplicate(&phys_region)?;

        if let Err(x) = dup.add_user(new_user) {
            warn!("{}", x);
        }

        Some(dup.virt_start_addr)
    })
}

pub fn kernel_print() {
    KERNEL_MAPPING_RECORD.read(|mr| mr.print());
}
