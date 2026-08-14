# Z_COM 文档

## 用户文档

| 文档 | 内容 |
|---|---|
| [使用与功能说明](user-guide.md) | 通信后端、报文显示、发送、配置、日志和多实例 |
| [Linux 构建与运行](linux.md) | Ubuntu 依赖、运行库、串口权限和 udev 规则 |
| [SEGGER / probe-rs 双 RTT 后端](rtt-backends.md) | 探针后端选择、兼容性和硬件验收 |
| [自定义 probe-rs MCU](custom-probe-rs-targets.md) | 使用 YAML 增加或修正 MCU 目标 |

## 开发文档

| 文档 | 内容 |
|---|---|
| [开发、检查与打包](development.md) | 环境、命令、目录结构、绿色版产物和维护规则 |
| [跨平台设计与验收](cross-platform.md) | Windows / Linux 实现原则、产物和最低验收矩阵 |
| [功能状态与路线图](roadmap.md) | 当前状态、待实机验收和暂不支持项 |
| [运行时更新](update-plan.md) | GitHub / Gitee 双源检查、下载校验、替换、回滚和验收范围 |

## 维护约定

- `README.md` 只保留项目入口信息，不堆放详细设计。
- 用户可见行为变化时更新 `user-guide.md`。
- 构建、目录或开发流程变化时更新 `development.md`。
- 跨平台约束或验收结果变化时更新 `cross-platform.md` 和对应平台文档。
- 新需求先写入 `roadmap.md`，实现并验证后再移动到已实现状态。
- RTT 后端或目标描述格式变化时同步更新对应专项文档。
