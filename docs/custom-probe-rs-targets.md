# 自定义 probe-rs MCU

应用启动时会创建 `config/probe_rs_targets` 目录。实际绝对路径可在“更多设置 → 调试探针 / RTT”中打开。该目录用于 CMSIS-DAP、ST-Link 等 probe-rs 后端；J-Link 使用 SEGGER 自带的目标数据库。

连接调试探针时，程序先加载 probe-rs 内置 Target Registry，再按文件名顺序加载该目录中的 `.yaml` 和 `.yml` 文件。同名 family 会覆盖内置定义，因此既可以增加新 MCU，也可以修正内置目标。

只用于 RTT 时不需要 Flash 算法。一个 Cortex-M4 目标的最小描述如下：

```yaml
name: MyChip Series
generated_from_pack: false

variants:
- name: MYCHIP123
  cores:
  - name: main
    type: armv7em
    core_access_options: !Arm
      ap: !v1 0
  memory_map:
  - !Ram
    name: RAM
    range:
      start: 0x20000000
      end: 0x20020000
    cores:
    - main
  flash_algorithms: []

flash_algorithms: []
```

保存后在设置中点击“重新加载”，目标名称 `MYCHIP123` 会进入候选列表；即使不手动重新加载，下一次连接也会读取最新文件。

需要根据芯片实际情况修改：

- `type`：例如 `armv6m`、`armv7m`、`armv7em`、`armv8m` 或 probe-rs 支持的其他核心类型。
- `ap`：目标核心所在的 ARM Access Port，普通单核 Cortex-M 常为 `0`。
- RAM `start`/`end`：RTT 控制块可能出现的有效 RAM 地址范围。

YAML 可以描述核心、RAM、JTAG 链和 Flash 算法，但不能定义任意厂商专用调试序列。如果芯片需要特殊上电、解锁或复位流程，仍需在 probe-rs 或本项目 Rust 后端中实现对应序列。
