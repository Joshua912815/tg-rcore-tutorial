set pagination off
set confirm off
set architecture riscv:rv64

file target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch3-observe
target remote :1234

break rust_main
break panic_stage1
break task::TaskControlBlock::handle_syscall

echo \nT2L21 GDB session ready.\n
echo Suggested next steps:\n
echo   1. continue\n
echo   2. inspect $pc / registers / scause at syscall boundary\n
echo   3. continue until panic_stage1 in crash-demo mode\n
