# stride 算法深入
## 实际情况是轮到 p1 执行吗？为什么？
不会，p2再次执行后stride会溢出变成4，还是小于p1的255
## 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。为什么？尝试简单说明（不要求严格证明）。
fork/spawn/exec应该要继承原有的stride，不然结论无法成立。

因为{进程优先级}>=2，所以PASS<=BigStride / 2;  
考虑到fork/spawn/exec均会继承原有的stride，因此可认为所有task是同步开始的，也就是初始stride相同；  

初始STRIDE_MAX–STRIDE_MIN<=BigStride/2成立，发生一次调度，有三个关键值{STRIDE_MAX}{STRIDE_MIN+PASS}{STRIDE_MIN_NEW}  
1. 如果{STRIDE_MAX}>={STRIDE_MIN+PASS}>={STRIDE_MIN_NEW}，那么STRIDE_MAX–STRIDE_MIN_NEW<=STRIDE_MAX–STRIDE_MIN<=BigStride/2成立
2. 如果{STRIDE_MIN+PASS}>={STRIDE_MAX}>={STRIDE_MIN_NEW}，那么{STRIDE_MIN+PASS}-{STRIDE_MIN_NEW}<=PASS<=BigStride/2
综上该结论永远成立

# 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

already borrowed: BorrowMutError翻看群友聊天记录
自己实现的copy_to_user需要获取pg_token,其中会获取current_task，sys_task_info
在调用之前已经获取过current_task

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

https://www.bookstack.cn/read/ucore_os_docs/lab6-lab6_3_6_1_basic_method.md
https://nankai.gitbook.io/ucore-os-on-risc-v64/lab6/tiao-du-suan-fa-kuang-jia#stride-suan-fa

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。




