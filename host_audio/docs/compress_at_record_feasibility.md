# 开录即压（算力 + 容量）可行性

## 结论

**可以。** 半率 IMA 已验证；**外存 replan v1** 后录音区 **14 MiB**；**下一档编解码目标为 Opus 16 kbps**（质量明显好于半率 IMA，体积通常更小或相当）。

## UI 占用

| 项 | 值 |
|----|-----|
| UI 配额 | 2 MiB |
| 现 CEMP rev20 | **396 629 B (~18.9%)** |
| 策略 | **不借给录音** |

## 外存 replan v1

`UI @0x0 2MiB + Recording @0x200000 14MiB`（见 Goal `companion-type-flash-replan-opus-codec-goal.md`）。

## 容量（14 MiB 录音）

| 模式 | 存盘速率 | 14 MiB 可录 | 10 min 体积 |
|------|----------|-------------|-------------|
| raw 16k PCM16 | 32 kB/s | ~7.6 min | 19.2 MiB ❌ |
| 半率 8k + IMA ~8:1 | ~4 kB/s | **~61 min** | ~2.3 MiB ✅ |
| **Opus 16 kbps** | ~2 kB/s | **~122 min** | ~1.2 MiB ✅ |
| Opus 24 kbps | ~3 kB/s | ~81 min | ~1.7 MiB ✅ |

平台：`COMPANION_RECORDING_REGION_BYTES=14MiB`，`OPUS_16KBPS_BYTES_PER_SEC`。

## 选型

- **基线：** 半率 IMA（已实现）  
- **下一默认目标：** Opus 16k mono（主机 A/B 见 `tools/run_companion_type_codec_ab_half_ima_vs_opus.ps1`）  
- **板端 Opus 实时编码：** stretch（X-CUBE-OPUS / fixed-point libopus）

## 会议纪要衔接

板：mic → encode → Flash → BLE → 主机 decode → 16 kHz WAV → ASR → L0/L1  
机械闭环必须 **扬声器外放 → 麦录**，file-fed 不算 PASS。
