# lab1实现报告

链接[https://learningos.cn/rCore-Camp-Guide-2024A/chapter3/5exercise.html](https://learningos.cn/rCore-Camp-Guide-2024A/chapter3/5exercise.html)

## 1. 实现的功能

### 实现系统调用 syscall ID: 410 (sys_task_info)

- 实现原理

在`struct TaskControlBlock`增加`counter: [u32; MAX_SYSCALL_NUM]`用来记录系统调用
次数; 增加`start_time: usize`记录task启动的时间, 从而计算出task运行的时间.

## 2. 简答

### 2.1 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

- sbi版本

```
[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0
```

- 程序出错行为

```rust
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
```

内核捕获到缺页、指令异常, 终止了当前任务的执行

### 2.2 深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题

- L40：刚进入 __restore 时，a0 代表了什么值。请指出 __restore 的两种使用情景。

- L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。



