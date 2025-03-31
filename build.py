import os

apps_dir = "../user/target/riscv64gc-unknown-none-elf/release/"
linker_script = "src/link_app.S"

# 创建一个简单的链接脚本，只包含ch3_taskinfo测试程序
with open(linker_script, 'w') as f:
    f.write('''
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad 1
    .quad app_0_start
    .quad app_0_end

    .section .data
    .global app_0_start
    .global app_0_end
app_0_start:
    .incbin "{}ch3_taskinfo"
app_0_end:
'''.format(apps_dir))

print("Generated simple linker script at {}".format(linker_script)) 