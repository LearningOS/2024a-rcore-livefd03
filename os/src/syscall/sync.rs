use core::ops::Add;

use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
/// get current task id
pub fn current_task_id() -> usize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let task_list = &process_inner.tasks;
    let tcb = current_task().unwrap();
    for (index, task) in task_list.iter().enumerate() {
        if let Some(task) = task {
            if Arc::ptr_eq(task, &tcb) {
                return index;
            }
        }
    }
    0
}
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

fn vec_le<T: Ord>(vec1: &Vec<T>, vec2: &Vec<T>) -> bool {
    assert_eq!(vec1.len(), vec2.len());
    for (x, y) in vec1.iter().zip(vec2.iter()) {
        if x > y {
            return false;
        }
    }
    true
}

fn vec_add<T: Add<Output = T> + Copy>(vec1: &mut Vec<T>, vec2: &Vec<T>) {
    assert_eq!(vec1.len(), vec2.len());
    for (x, y) in vec1.iter_mut().zip(vec2.iter()) {
        *x = *x + *y;
    }
}

fn test_mutex_dead_lock(task_id: usize, mutex_id: usize) -> bool {
    let pcb = current_process();
    let pcb_inner = pcb.inner_exclusive_access();

    let mut work: Vec<usize> = pcb_inner
        .mutex_list
        .iter()
        .map(|item| {
            if let Some(mutex) = item {
                if mutex.locked() {
                    0
                } else {
                    1
                }
            } else {
                0
            }
        })
        .collect();
    let allocation = pcb_inner.mutex_alloction.clone();
    let mut request = vec![vec![0; allocation[0].len()]; allocation.len()];
    request[task_id][mutex_id] = 1;
    let mut finish: Vec<bool> = pcb_inner.tasks.iter().map(|item| item.is_none()).collect();

    let mut change = true;
    while change {
        change = false;
        for (task_id_t, task_allocation) in allocation.iter().enumerate() {
            if finish[task_id_t] == false && vec_le(&request[task_id_t], &work) {
                finish[task_id_t] = true;
                vec_add(&mut work, task_allocation);
                change = true;
            }
        }
    }
    for flag in finish {
        if flag == false {
            return true;
        }
    }
    false
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        process_inner
            .mutex_alloction
            .iter_mut()
            .for_each(|request| {
                request[id] = 0;
            });
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner
            .mutex_alloction
            .iter_mut()
            .for_each(|request| {
                request.push(0);
            });
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let current_task_id = current_task_id();
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    if test_mutex_dead_lock(current_task_id, mutex_id) {
        return -0xDEAD;
    }
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.mutex_alloction[current_task_id][mutex_id] += 1;
    0
}

/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let current_task_id = current_task_id();
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.mutex_alloction[current_task_id][mutex_id] -= 1;
    0
}
fn test_sem_dead_lock(task_id: usize, sem_id: usize) -> bool {
    let pcb = current_process();
    let pcb_inner = pcb.inner_exclusive_access();

    // let mut work: Vec<isize> = pcb_inner
    //     .semaphore_list
    //     .iter()
    //     .map(|item| {
    //         if let Some(semaphore) = item {
    //             semaphore.count()
    //         } else {
    //             0
    //         }
    //     })
    //     .collect();
    let mut work = pcb_inner.semaphore_available.clone();
    let allocation = pcb_inner.semaphore_alloction.clone();
    let mut request = pcb_inner.semaphore_need.clone();
    request[task_id][sem_id] += 1;
    let mut finish: Vec<bool> = pcb_inner.tasks.iter().map(|item| item.is_none()).collect();

    let mut change = true;
    while change {
        change = false;
        for (task_id_t, task_allocation) in allocation.iter().enumerate() {
            if finish[task_id_t] == false && vec_le(&request[task_id_t], &work) {
                finish[task_id_t] = true;
                vec_add(&mut work, task_allocation);
                change = true;
            }
        }
    }
    for flag in finish {
        if flag == false {
            return true;
        }
    }
    false
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        process_inner.semaphore_available[id] = res_count as isize;
        process_inner
            .semaphore_alloction
            .iter_mut()
            .for_each(|request| {
                request[id] = 0;
            });
        process_inner.semaphore_need.iter_mut().for_each(|request| {
            request[id] = 0;
        });
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_available.push(res_count as isize);
        process_inner
            .semaphore_alloction
            .iter_mut()
            .for_each(|request| {
                request.push(0);
            });
        process_inner.semaphore_need.iter_mut().for_each(|request| {
            request.push(0);
        });
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let current_task_id = current_task_id();
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.semaphore_alloction[current_task_id][sem_id] -= 1;
    process_inner.semaphore_available[sem_id] += 1;
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let current_task_id = current_task_id();
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    if test_sem_dead_lock(current_task_id, sem_id) {
        return -0xDEAD;
    }
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.semaphore_need[current_task_id][sem_id] += 1;
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.semaphore_alloction[current_task_id][sem_id] += 1;
    process_inner.semaphore_available[sem_id] -= 1;
    process_inner.semaphore_need[current_task_id][sem_id] -= 1;
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    -1
}
