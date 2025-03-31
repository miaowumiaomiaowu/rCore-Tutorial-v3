import os
import subprocess

# 设置目录路径
user_dir = "../user"
target_dir = f"{user_dir}/target/riscv64gc-unknown-none-elf/release"
bin_dir = f"{user_dir}/src/bin"
linker_script = "src/link_app.S"

# 确保目标目录存在
os.makedirs(target_dir, exist_ok=True)

# 获取所有的用户程序
apps = []
for file in os.listdir(bin_dir):
    if file.endswith(".rs"):
        apps.append(file[:-3])  # 移除.rs后缀

if not apps:
    print("没有找到用户程序！")
    exit(1)

# 排序应用程序名称以确保顺序一致
apps.sort()

# 确保ch3_taskinfo是第一个程序
if "ch3_taskinfo" in apps:
    apps.remove("ch3_taskinfo")
    apps = ["ch3_taskinfo"] + apps

print(f"找到 {len(apps)} 个用户程序：{apps}")

# 生成link_app.S文件
with open(linker_script, 'w') as f:
    f.write(f'''
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {len(apps)}
''')
    
    # 添加应用程序起始位置指针
    for i, app in enumerate(apps):
        f.write(f'    .quad app_{i}_start\n')
    f.write(f'    .quad app_{len(apps) - 1}_end\n\n')
    
    # 添加应用程序数据
    for i, app in enumerate(apps):
        f.write(f'''
    .section .data
    .global app_{i}_start
    .global app_{i}_end
app_{i}_start:
    .incbin "{target_dir}/{app}"
app_{i}_end:
''')

print(f"生成链接脚本 {linker_script} 完成！")
print(f"包含的应用程序：{apps}") 