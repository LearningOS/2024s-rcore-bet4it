# Lab 2

整体代码也没什么问题。不过在`ci-user`下运行`make test`会修改主目录、`ci-user`目录、`ci-user/user`目录下的一堆文件，这些文件还没被`git`忽略掉，`git status`一堆红，简直令人窒息。

主目录下的文件被改了之后还没法切换到新分支，需要`git restore .`一下才可以。`ch1`切换到`ch2`还需要`rm os/build.rs`。
