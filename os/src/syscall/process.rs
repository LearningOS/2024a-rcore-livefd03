//! Process management syscalls
use core::{mem::size_of, slice};

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{translated_byte_buffer, MapPermission, VirtAddr, VirtPageNum},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus, TASK_MANAGER,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

fn copy_to_user(src: usize, dst: usize, size: usize) {
    let pg_token = current_user_token();
    let mut dst_buf = translated_byte_buffer(pg_token, dst as *const u8, size);
    let src_slice = unsafe { slice::from_raw_parts(src as *const u8, size) };
    let mut count = 0;
    for buf_slice in dst_buf.iter_mut() {
        let target_len = buf_slice.len();
        buf_slice.copy_from_slice(&src_slice[count..count + target_len]);
        count += target_len
    }
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let ts = TimeVal {
        sec: get_time_us() / 1_000_000,
        usec: get_time_us() % (1_000_1000),
    };
    copy_to_user(
        (&ts) as *const TimeVal as usize,
        _ts as usize,
        size_of::<TimeVal>(),
    );
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let mut ti = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: [0; MAX_SYSCALL_NUM],
        time: 0,
    };
    TASK_MANAGER.get_task_info(&mut ti);
    copy_to_user(
        (&ti) as *const TaskInfo as usize,
        _ti as usize,
        size_of::<TaskInfo>(),
    );
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let left_vaddr = VirtAddr::from(_start);
    let right_vaddr = VirtAddr::from(_start + _len);
    if !left_vaddr.aligned() {
        return -1;
    }
    if _port & !0x7 != 0 || _port == 0 {
        return -1;
    }

    let left_vpn = VirtPageNum::from(left_vaddr);
    let right_vpn = right_vaddr.ceil();
    let permission: MapPermission =
        MapPermission::from_bits_truncate(((_port as u8) << 1) | (1 << 4));

    // get current task memset
    let mut manager = TASK_MANAGER.inner.exclusive_access();
    let current = manager.current_task;
    let memset = &mut manager.tasks[current].memory_set;

    for map_area in memset.areas.iter() {
        if map_area.vpn_range.get_start() >= right_vpn || left_vpn >= map_area.vpn_range.get_end() {
            continue;
        }
        return -1;
    }
    memset.insert_framed_area(left_vaddr, right_vpn.into(), permission);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let left_vaddr = VirtAddr::from(_start);
    let right_vaddr = VirtAddr::from(_start + _len);
    if !left_vaddr.aligned() {
        return -1;
    }
    let left_vpn = VirtPageNum::from(left_vaddr);
    let right_vpn = right_vaddr.ceil();
    // get current task memset
    let mut manager = TASK_MANAGER.inner.exclusive_access();
    let current = manager.current_task;
    let memset = &mut manager.tasks[current].memory_set;
    let mut area_index = None;
    for (index,map_area) in memset.areas.iter_mut().enumerate() {
        if map_area.vpn_range.get_start() == left_vpn && right_vpn == map_area.vpn_range.get_end() {
            map_area.unmap(&mut memset.page_table);
            area_index = Some(index);
        }
    }
    if let Some(index) = area_index{
        memset.areas.remove(index);
        0
    }
    else{
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
