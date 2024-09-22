#[inline(always)]
pub const fn is_aligned(value: usize, alignment: usize) -> bool {
    assert!(alignment.is_power_of_two());

    (value & (alignment - 1)) == 0
}

#[inline(always)]
pub const fn align_down(value: usize, alignment: usize) -> usize {
    assert!(alignment.is_power_of_two());

    value & !(alignment - 1)
}

#[inline(always)]
pub const fn align_up(value: usize, alignment: usize) -> usize {
    assert!(alignment.is_power_of_two());

    (value + alignment - 1) & !(alignment - 1)
}

pub const fn size_human_readable_ceil(size: usize) -> (usize, &'static str) {
    const KIB: usize = 1024;
    const MIB: usize = 1024 * KIB;
    const GIB: usize = 1024 * MIB;

    if (size / GIB) > 0 {
        (size.div_ceil(GIB), "GiB")
    } else if (size / MIB) > 0 {
        (size.div_ceil(MIB), "MiB")
    } else if (size / KIB) > 0 {
        (size.div_ceil(KIB), "KiB")
    } else {
        (size, "B")
    }
}
