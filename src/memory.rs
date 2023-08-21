use crate::*;

/// Some of the code in this function is based on sonodima's code from
/// https://github.com/sonodima/aobscan/blob/master/src/pattern.rs
pub fn aob_scan_single_threaded(
    pattern: &str,
    data: &[u8],
    return_on_first: bool,
) -> Result<Vec<usize>> {
    let mut signature: Vec<u8> = vec![];
    let mut mask: Vec<bool> = vec![];

    for pair in pattern.split_whitespace() {
        if pair == "?" || pair == "??" {
            mask.push(false);
            signature.push(0);
        } else {
            mask.push(true);
            signature.push(u8::from_str_radix(pair, 16)?);
        }
    }

    let mut start_offset = mask.iter().take_while(|&&x| x == false).count();
    let end_offset = mask.iter().rev().take_while(|&&x| x == false).count();

    if start_offset != mask.len() {
        signature = signature[start_offset..signature.len() - end_offset].to_vec();
        mask = mask[start_offset..mask.len() - end_offset].to_vec();
    } else {
        start_offset = 0;
    }

    let first_byte = signature[0];
    let first_mask = mask[0];

    let mut address_array: Vec<usize> = Vec::new();

    for i in 0..data.len() - signature.len() {
        if data[i] != first_byte && first_mask {
            continue;
        }

        if {
            let data = &data[i..];
            let mut status = true;
            for (i, sig) in signature.iter().enumerate() {
                if !mask[i] {
                    continue;
                }

                if data[i] != *sig {
                    status = false;
                    break;
                }
            }
            status
        } {
            address_array.push(i - start_offset);
            if return_on_first {
                break;
            }
        }
    }
    address_array.sort();
    Ok(address_array)
}

/// Some of the code in this function is based on sonodima's code from
/// https://github.com/sonodima/aobscan/blob/master/src/pattern.rs
/// https://github.com/sonodima/aobscan/blob/master/src/builder.rs
pub fn aob_scan_multi_threaded(
    pattern: &str,
    data: &[u8],
    return_on_first: bool,
    thread_count: usize,
) -> Result<Vec<usize>> {
    if pattern.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Pattern cannot be empty",
        )));
    }

    if data.len() == 0 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Data cannot be empty",
        )));
    }

    if thread_count < 2 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Thread count must be greater than one",
        )));
    }

    let mut signature: Vec<u8> = Vec::<u8>::new();
    let mut mask: Vec<bool> = Vec::<bool>::new();

    for pair in pattern.split_whitespace() {
        if pair == "?" || pair == "??" {
            mask.push(false);
            signature.push(0);
        } else {
            mask.push(true);
            signature.push(u8::from_str_radix(pair, 16)?);
        }
    }

    let mut start_offset = mask.iter().take_while(|&&x| x == false).count();
    let end_offset = mask.iter().rev().take_while(|&&x| x == false).count();

    if start_offset != mask.len() {
        signature = signature[start_offset..signature.len() - end_offset].to_vec();
        mask = mask[start_offset..mask.len() - end_offset].to_vec();
    } else {
        start_offset = 0;
    }

    let running_thread_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let found = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let address_array = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));

    let signature = &signature;
    let mask = &mask;

    std::thread::scope(|scope| {
        for index in 0..thread_count {
            let range = {
                let data_size = data.len();
                let chunks = thread_count;
                let overlap = signature.len() - 1;
                let chunk_size = data_size / chunks;
                let remainder = data_size % chunks;

                let start = index * chunk_size;

                let mut end = start + chunk_size + if index == chunks - 1 { remainder } else { 0 };

                let start = start - if start >= overlap { overlap } else { 0 };

                end = end
                    + if end < data_size - overlap {
                        overlap
                    } else {
                        0
                    };

                (start, end)
            };

            let running_thread_count = running_thread_count.clone();
            let finished = finished.clone();
            let found = found.clone();

            running_thread_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            let addres_array = address_array.clone();

            scope.spawn(move || {
                let data = &data[range.0..range.1];

                let length = data.len() - signature.len();

                let first_byte = signature[0];
                let first_mask = mask[0];

                let mut found_in = false;

                for i in 0..length {
                    if finished.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }

                    if data[i] != first_byte && first_mask {
                        continue;
                    }

                    if {
                        let data = &data[i..];
                        let mut status = true;
                        for (i, sig) in signature.iter().enumerate() {
                            if !mask[i] {
                                continue;
                            }

                            if data[i] != *sig {
                                status = false;
                                break;
                            }
                        }
                        status
                    } {
                        found_in = true;
                        if let Ok(mut val) = addres_array.lock() {
                            val.push(range.0 + i - start_offset);
                        } else {
                            return;
                        }
                        if return_on_first {
                            finished.store(true, std::sync::atomic::Ordering::Relaxed);
                            break;
                        }
                    }
                }

                if found_in {
                    found.store(true, std::sync::atomic::Ordering::SeqCst);
                }

                running_thread_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            });
        }
    });

    while running_thread_count.load(std::sync::atomic::Ordering::SeqCst) != 0 {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    found.load(std::sync::atomic::Ordering::SeqCst);
    let mut address_array = if let Ok(val) = address_array.lock() {
        val.to_vec()
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mutex lock failed",
        )));
    };
    address_array.sort();
    Ok(address_array)
}

pub fn read_memory(
    process_handle: *mut core::ffi::c_void,
    address: *const core::ffi::c_void,
    size: usize,
) -> Result<Vec<u8>> {
    unsafe {
        let result = VirtualQueryEx(
            process_handle,
            address,
            &mut MemoryBasicInformation {
                ..core::mem::zeroed()
            },
            core::mem::size_of::<MemoryBasicInformation>(),
        );

        if result == 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("VirtualQueryEx failed with return value: {result:X}"),
            )));
        }

        let mut old_protect = 0u32;

        let mut new_protect = 4u32;

        let result = VirtualProtectEx(
            process_handle,
            address,
            core::mem::size_of::<*mut core::ffi::c_void>(),
            new_protect,
            &mut old_protect,
        );

        if result == 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("VirtualProtectEx failed with return value: {result:X}"),
            )));
        }

        let mut buffer: Vec<u8> = Vec::new();

        buffer.resize(size, 0u8);

        let result = ReadProcessMemory(
            process_handle,
            address,
            buffer.as_mut_ptr().cast(),
            size,
            core::ptr::null_mut(),
        );

        if result == 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("ReadProcessMemory failed with return value: {result:X}"),
            )));
        }

        let result = VirtualProtectEx(
            process_handle,
            address,
            core::mem::size_of::<*mut core::ffi::c_void>(),
            old_protect,
            &mut new_protect,
        );

        if result == 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("VirtualProtectEx failed with return value: {result:X}"),
            )));
        }

        Ok(buffer)
    }
}

pub fn write_memory(
    process_handle: *mut core::ffi::c_void,
    address: *mut core::ffi::c_void,
    buffer: &mut Vec<u8>,
) -> Result<()> {
    unsafe {
        let result = WriteProcessMemory(
            process_handle,
            address,
            buffer.as_ptr().cast(),
            buffer.len(),
            core::ptr::null_mut(),
        );
        if result == 0 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("WriteProcessMemory failed with return value: {result:X}"),
            )));
        }

        Ok(())
    }
}
