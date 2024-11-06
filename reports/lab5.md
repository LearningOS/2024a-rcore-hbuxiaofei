
# lab5实现报告

[题目链接](https://learningos.cn/rCore-Camp-Guide-2024A/chapter8/5exercise.html)

## 1. 实现的功能

### 实现了是否开启死锁检查系统调用`enable_deadlock_detect`及死锁检查的算法

- 实现原理


`enable_deadlock_detect`实现是为每个进程增加一个标记`enable_dld`来控制是否开启死锁检测,
死锁检测算法是按照题目中给出的[算法描述](https://learningos.cn/rCore-Camp-Guide-2024A/chapter8/5exercise.html)严格实现的, 其实实际上可以简化很多, 例如如果将
锁比作临界资源, 一个进程一次获取锁只相当于获取了数量为1的资源, 是不可能同时占有多于1个
临界资源的.


## 2. 问答作业


### 2.1 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

需要回收的资源: 映射的地址空间需要释放, 文件描述符需要释放.

其他的TaskControlBlock以及一些缓存,由于rust的自动drop机制不用回收.



### 2.2 对比以下两种 Mutex.unlock 的实现，二者有什么区别？这些区别可能会导致什么问题？

```rust
impl Mutex for Mutex1 {
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        mutex_inner.locked = false;
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        }
    }
}

impl Mutex for Mutex2 {
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
```

实现区别`mutex_inner.locked`设置false的位置不同.

第一种实现, 如果2个线程同时lock, 1个线程unlock后, 第2个线程再执行unlock会导致断言失败.

## 3. 荣誉准则

- 1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

> 《你交流的对象说明》

- 2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

> 《你参考的资料说明》

- 3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

- 4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
