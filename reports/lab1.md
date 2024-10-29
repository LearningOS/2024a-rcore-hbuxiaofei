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

### 2.2 深入理解 trap.S 中两个函数`__alltraps` 和` __restore` 的作用，并回答如下问题

- L40：刚进入`__restore` 时，a0 代表了什么值。请指出`__restore` 的两种使用情景。

a0值: 上课的ppt中有`mv sp a0`, 实际代码种没有了这个代码, ppt中a0的含义应该是内核栈顶

2种使用场景: (1) 应用程序最初初始化,内核栈保存trap上下文 (2) trap到了内核返回后,恢复上下文

- L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

```asm
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```

trap上下文定义如下, 可以看出sstatus偏移为32****8, sepc偏移为33*8, sp(x2)偏移为2*8,
```rust
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}
```
通过t0 t1 t2 临时寄存器, 这段代码恢复sstatus, sepc, sscratch的值. 其中
sstatus会有特权级信息, sepc表示trap处理完后下一条指令地址, sscratch表示用户态栈指针.

- L50-L56：为何跳过了 x2 和 x4？

```asm
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```

x2为sp, 通过指令`csrrw sp, sscratch, sp`恢复; x4为tp, 不支持线程, 暂时用不到


- L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```asm
csrrw sp, sscratch, sp
```

执行这条指令后, sp 将指向用户栈, sscratch 指向原来的内核栈


- `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

执行sret后发生状态切换. 执行完sret后, sstatus寄存器中的SPP位被设置为用户态;
sepc 的值加载到 PC 中, 使得程序从发生异常的指令继续执行.

- L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

```asm
csrrw sp, sscratch, sp
```

执行这条指令后, sp 将指向内核栈, sscratch 指向原来的用户栈

- 从 U 态进入 S 态是哪一条指令发生的？

```asm
csrrw sp, sscratch, sp
```
这条指令将当前的栈指针sp(用户栈)写入sscratch寄存器,并将sscratch(内核栈)加载到sp中,
用户态的上下文被保存, 控制权转移到了S态.

## 3. 荣誉准则

- 1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

> 《你交流的对象说明》

- 2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

> 《你参考的资料说明》

- 3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

- 4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
