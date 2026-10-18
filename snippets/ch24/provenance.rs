// strict provenance：把「地址」和「provenance」分开操作。
fn main() {
    let arr = [10u32, 20, 30, 40];
    let base: *const u32 = &raw const arr[0];

    println!("base.addr() = {:#x}", base.addr());

    // 保留 provenance 的加偏移：等价于 wrapping_offset。
    let second = base.with_addr(base.addr() + size_of::<u32>());
    println!("自动解引用第二个元素 = {}", unsafe { *second });

    // map_addr 适合「只动地址、不动来历」的位运算。
    let tagged = base.map_addr(|a| a | 1);
    let untagged = tagged.map_addr(|a| a & !1);
    println!("标记前 {:#x}，标记后 {:#x}", base.addr(), tagged.addr());
    assert_eq!(base.addr(), untagged.addr());
    assert_eq!(base.addr() | 1, tagged.addr());

    // 显式暴露 provenance，之后再凭地址取回。
    let exposed: usize = base.expose_provenance();
    let round_trip: *const u32 = std::ptr::with_exposed_provenance::<u32>(exposed);
    println!("往返之后读到 = {}", unsafe { *round_trip });

    // 没有 provenance 的指针：地址可信，但既不是分配、也不能解引用。
    let nowhere: *const u32 = std::ptr::without_provenance(0x1000);
    println!(
        "nowhere.addr() = {:#x}，is_null = {}",
        nowhere.addr(),
        nowhere.is_null()
    );
}
