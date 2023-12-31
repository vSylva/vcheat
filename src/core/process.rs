macro_rules! location {
    () => {
        format!("[{}:{}]", file!(), line!())
    };

    ($val:literal) => {
        format!("[{}:{}]\t\"{}\"", file!(), line!(), $val)
    };

    ($($val:expr),*) => {
        {
            let mut text =  format!("[{}:{}]", file!(), line!());

            text.push('\t');

            text.push('\"');

            $(
                text += &format!("{} = {:?}", stringify!($val), $val);

                text.push('\t');
            )*

            text = text.trim_end().to_string();

            text.push('\"');

            text
        }
    };
}

use crate::{core, ffi};

pub(crate) unsafe fn open_process(process_id: u32) -> Result<*mut ::core::ffi::c_void, String> {
    let process_handle: *mut ::core::ffi::c_void = ffi::OpenProcess(0x1F0FFF, 0, process_id);

    if process_handle.is_null() {
        return Err(location!());
    }

    Ok(process_handle)
}

pub(crate) unsafe fn close_handle(handle: *mut ::core::ffi::c_void) -> Result<(), String> {
    if handle.is_null() {
        return Err(location!());
    }

    let result = ffi::CloseHandle(handle);

    if result == 0 {
        return Err(location!());
    }

    Ok(())
}

pub(crate) unsafe fn close_handle_unchecked(handle: *mut ::core::ffi::c_void) -> i32 {
    ffi::CloseHandle(handle)
}

pub(crate) unsafe fn is_wow64_process(
    process_handle: *mut ::core::ffi::c_void,
) -> Result<bool, String> {
    if process_handle.is_null() {
        return Err(location!());
    }

    let mut is_wow64: i32 = 0;

    let result: i32 = ffi::IsWow64Process(process_handle, &mut is_wow64);

    if result == 0 {
        return Err(location!());
    }

    Ok(is_wow64 != 0)
}

pub(crate) unsafe fn get_processes_info() -> Result<Vec<core::ProcessInformation>, String> {
    let snapshot_handle: *mut ::core::ffi::c_void = ffi::CreateToolhelp32Snapshot(0x2, 0x0);

    if snapshot_handle as i32 == -1 {
        return Err(location!(snapshot_handle));
    }

    let mut process_entry: ffi::ProcessEntry32W = ::core::mem::zeroed::<ffi::ProcessEntry32W>();

    process_entry.dw_size = ::core::mem::size_of::<ffi::ProcessEntry32W>() as u32;

    let result: i32 = ffi::Process32FirstW(snapshot_handle, &mut process_entry);

    if result == 0 {
        close_handle(snapshot_handle)?;

        return Err(location!());
    }

    let mut process_entry_array: Vec<ffi::ProcessEntry32W> = Vec::<ffi::ProcessEntry32W>::new();

    process_entry_array.push(process_entry.to_owned());

    while ffi::Process32NextW(snapshot_handle, &mut process_entry) != 0 {
        process_entry_array.push(process_entry.to_owned());
    }

    close_handle(snapshot_handle)?;

    let mut process_info_array: Vec<core::ProcessInformation> =
        Vec::<core::ProcessInformation>::new();

    for process_entry in process_entry_array {
        process_info_array.push(core::ProcessInformation {
            id: process_entry.th32_process_id,
            thread_count: process_entry.cnt_threads,
            parent_process_id: process_entry.th32_parent_process_id,
            base_priority_class: process_entry.pc_pri_class_base,
            name: {
                let result: ::std::ffi::OsString =
                    ::std::os::windows::prelude::OsStringExt::from_wide(&process_entry.sz_exe_file);

                match result.to_str() {
                    Some(some) => some.trim_end_matches('\0').to_string(),

                    None => {
                        close_handle(snapshot_handle)?;

                        return Err(location!());
                    }
                }
            },
        });
    }

    Ok(process_info_array)
}

pub(crate) unsafe fn get_process_info(
    process_name: &str,
) -> Result<core::ProcessInformation, String> {
    if process_name.is_empty() {
        return Err(location!());
    }

    let process_name: String = process_name.to_lowercase();

    let snapshot_handle: *mut ::core::ffi::c_void = ffi::CreateToolhelp32Snapshot(0x2, 0x0);

    if snapshot_handle as i32 == -1 {
        return Err(location!(snapshot_handle));
    }

    let mut process_entry: ffi::ProcessEntry32W = ::core::mem::zeroed::<ffi::ProcessEntry32W>();

    process_entry.dw_size = ::core::mem::size_of::<ffi::ProcessEntry32W>() as u32;

    let result: i32 = ffi::Process32FirstW(snapshot_handle, &mut process_entry);

    if result == 0 {
        close_handle(snapshot_handle)?;

        return Err(location!());
    }

    let process_entry_name: String = {
        let result: ::std::ffi::OsString =
            ::std::os::windows::prelude::OsStringExt::from_wide(&process_entry.sz_exe_file);

        match result.to_str() {
            Some(some) => some.trim_end_matches('\0').to_string(),

            None => {
                close_handle(snapshot_handle)?;

                return Err(location!());
            }
        }
    };

    if process_entry_name.to_lowercase() == process_name {
        close_handle(snapshot_handle)?;

        return Ok(core::ProcessInformation {
            id: process_entry.th32_process_id,
            thread_count: process_entry.cnt_threads,
            parent_process_id: process_entry.th32_parent_process_id,
            base_priority_class: process_entry.pc_pri_class_base,
            name: process_entry_name,
        });
    }

    while ffi::Process32NextW(snapshot_handle, &mut process_entry) != 0 {
        let process_entry_name: String = {
            let result: ::std::ffi::OsString =
                ::std::os::windows::prelude::OsStringExt::from_wide(&process_entry.sz_exe_file);

            match result.to_str() {
                Some(some) => some.trim_end_matches('\0').to_string(),
                None => return Err(location!()),
            }
        };

        if process_entry_name.to_lowercase() == process_name {
            close_handle(snapshot_handle)?;

            return Ok(core::ProcessInformation {
                id: process_entry.th32_process_id,
                thread_count: process_entry.cnt_threads,
                parent_process_id: process_entry.th32_parent_process_id,
                base_priority_class: process_entry.pc_pri_class_base,
                name: process_entry_name,
            });
        }
    }

    close_handle(snapshot_handle)?;

    Err(location!())
}

pub(crate) unsafe fn nt_get_processes_info() -> Result<Vec<core::SystemProcessInformation>, String>
{
    let mut return_length: u32 = 0;

    let _: i32 = ffi::NtQuerySystemInformation(5, ::core::ptr::null_mut(), 0, &mut return_length);

    let mut buffer: Vec<u8> = vec![0; return_length as usize * 2];

    let result: i32 = ffi::NtQuerySystemInformation(
        5,
        buffer.as_mut_ptr().cast(),
        return_length * 2,
        &mut return_length,
    );

    if result != 0 {
        return Err(location!(result));
    }

    let mut process_info_array: Vec<ffi::SystemProcessInformation> =
        Vec::<ffi::SystemProcessInformation>::new();

    let mut current_offset: isize = 0;

    let mut process_info: ffi::SystemProcessInformation =
        ::core::ptr::read::<ffi::SystemProcessInformation>(buffer.as_ptr().cast());

    while process_info.next_entry_offset != 0 {
        if process_info.unique_process_id != 0 {
            process_info_array.push(process_info.clone());
        }

        current_offset += process_info.next_entry_offset as isize;

        if buffer.as_ptr().offset(current_offset).is_null() {
            break;
        }

        process_info = ::core::ptr::read::<ffi::SystemProcessInformation>(
            buffer.as_ptr().offset(current_offset).cast(),
        );
    }

    let mut nt_process_info_array: Vec<core::SystemProcessInformation> =
        Vec::<core::SystemProcessInformation>::new();

    for p in process_info_array {
        nt_process_info_array.push(core::SystemProcessInformation {
            thread_count: p.number_of_threads,
            name: {
                let name_data: &[u16] = ::std::slice::from_raw_parts(
                    p.image_name.buffer,
                    (p.image_name.length) as usize / 2,
                );

                let result: ::std::ffi::OsString =
                    ::std::os::windows::prelude::OsStringExt::from_wide(&name_data);

                match result.to_str() {
                    Some(some) => some.trim_end_matches('\0').to_string(),
                    None => return Err(location!()),
                }
            },
            base_priority_class: p.base_priority,
            id: p.unique_process_id,
            handle_count: p.handle_count,
            session_id: p.session_id,
            peak_virtual_size: p.peak_virtual_size,
            virtual_size: p.virtual_size,
            peak_working_set_size: p.peak_working_set_size,
            working_set_size: p.working_set_size,
            quota_paged_pool_usage: p.quota_paged_pool_usage,
            quota_non_paged_pool_usage: p.quota_non_paged_pool_usage,
            pagefile_usage: p.pagefile_usage,
            peak_pagefile_usage: p.peak_pagefile_usage,
            private_page_count: p.private_page_count,
        });
    }

    Ok(nt_process_info_array)
}

pub(crate) unsafe fn alloc_console() -> Result<(), String> {
    if ffi::AllocConsole() == 0 {
        return Err(location!());
    }

    Ok(())
}

pub(crate) unsafe fn alloc_console_unchecked() -> i32 {
    ffi::AllocConsole()
}

pub(crate) unsafe fn free_console() -> Result<(), String> {
    if ffi::FreeConsole() == 0 {
        return Err(location!());
    }

    Ok(())
}

pub(crate) unsafe fn free_console_unchecked() -> i32 {
    ffi::FreeConsole()
}

pub(crate) unsafe fn set_console_mode(
    standard_handle: u32,
    console_mode: u32,
) -> Result<(), String> {
    let standard_handle: *mut ::core::ffi::c_void = ffi::GetStdHandle(standard_handle);

    if standard_handle as isize == -1 {
        return Err(location!());
    }

    let mut current_console_mode: u32 = 0;

    if 0 == ffi::GetConsoleMode(standard_handle, &mut current_console_mode) {
        return Err(location!());
    }

    if 0 == ffi::SetConsoleMode(standard_handle, current_console_mode | console_mode) {
        return Err(location!());
    }

    Ok(())
}

pub(crate) unsafe fn set_console_colors() -> Result<(), String> {
    set_console_mode(
        crate::core::standard_handle::OUTPUT_HANDLE,
        crate::core::console_mode::ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    )
}
