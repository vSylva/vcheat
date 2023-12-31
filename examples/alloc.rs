use vcheat::{memory, process, *};

fn main() {
    let process_handle = process::open_process(::std::process::id()).unwrap();

    let size = 1024;

    '_std_alloc: {
        let standard_allocated_address = memory::standard_alloc(size).unwrap();

        let standard_query_info =
            memory::virtual_query(process_handle, standard_allocated_address.cast()).unwrap();

        assert_eq!(standard_query_info.page_protect, page_protect::READ_WRITE);

        memory::standard_free(standard_allocated_address, size).unwrap();
    }

    '_win_alloc: {
        let virtual_allocated_address = memory::virtual_alloc(
            ::core::ptr::null_mut(),
            size,
            mem_allocation::RESERVE | mem_allocation::COMMIT,
            page_protect::EXECUTE_READ,
        )
        .unwrap();

        let query_info =
            memory::virtual_query(process_handle, virtual_allocated_address.cast()).unwrap();

        assert_eq!(query_info.page_protect, page_protect::EXECUTE_READ);

        memory::virtual_free(virtual_allocated_address, 0, mem_free::RELEASE).unwrap();
    }

    process::close_handle(process_handle).unwrap();
}
