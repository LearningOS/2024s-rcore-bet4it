# Lab 3

这次改了`rustsbi-qemu.bin`还是没办法用高版本`QEMU`运行，只好自己编译`7.0.0`版本的`QEMU`。在`Arch Linux`上使用`./configure --target-list=riscv64-softmmu --disable-werror --disable-bpf`可以编译成功。

不过`rCore-Tutorial-v3`的`ch3`依然可以在新版`QEMU`上运行，说明解决方案已经有了只不过没合并过来。主要是我自己也没时间研究就没管了。

这个实验需要记录每个进程运行过程中的信息，所以把相关信息存在`TCB`里比较合适。然后就需要通过`TaskManager`修改`TCB`里面的内容，已有的代码中是把这些修改包装成几个全局的函数，我也就照葫芦画瓢做了，不过感觉直接调用`TaskManager`中的函数是不是观感更好一些。

注意这里`TaskControlBlock`设了`#[derive(Copy, Clone)]`，所以如果想直接修改其中的元素需要获取`&mut`这样的引用，如果直接用赋值获取`TCB`的话就修改不到原来数据结构的内容了。我觉得像`TCB`这种全局唯一的数据结构不应该设`#[derive(Copy, Clone)]`，这样编译器就能帮忙检查出来一些错误的使用。

还有就是`TaskInfo`有一项是系统调用时刻距离任务第一次被调度时刻的时长，因此需要记录任务第一次被调度的时候的时间。不过这个“第一次被调度”的点不太好找，除了`run_first_task`肯定是第一个任务第一次被调度的时间外，其他任务的第一次被调用和后续被调用都在`run_next_task`中，所以我这里加了个判断`if inner.tasks[next].start_time == 0`来看是不是第一次被调用。
