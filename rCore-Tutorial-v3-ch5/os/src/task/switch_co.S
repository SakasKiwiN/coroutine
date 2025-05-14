# os/src/task/switch_co.S
.altmacro
.macro SAVE_SN n
    sd s\n, \n*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, \n*8(a1)
.endm
    .section .text
    .globl __switch_co
__switch_co:
    # a0: &current_context (即将被切换出的协程上下文的地址)
    # a1: &next_context (即将被切换进的协程上下文的地址)

    # 保存当前寄存器
    sd ra, 0(a0)
    sd sp, 8(a0)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n+1
    .endr

    # 加载下一个协程的寄存器
    ld ra, 0(a1)
    ld sp, 8(a1)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n+1
    .endr

    # 返回到下一个协程
    ret