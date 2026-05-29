"""
日志转换工具：将 MIX 格式的日志文件转换为 HEX-only 或 ASCII-only 格式。

用法:
    python log_converter.py <input_file> --format hex
    python log_converter.py <input_file> --format ascii
    python log_converter.py <input_file> --format both
    python log_converter.py --dir logs/ --format both
"""

import argparse
import os
import re
import sys
from pathlib import Path


_TS_RE = re.compile(r'^\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}\]')


def _is_timestamp(line: str) -> bool:
    return line.startswith('[') and bool(_TS_RE.match(line))


def parse_entries(text: str) -> list[list[str]]:
    """将 MIX 日志按条目拆分。

    条目类型:
    1. 事件条目 (单行): [timestamp] 内容
    2. 数据条目: [timestamp] + ← HEX: (单行) + ← ASCII: (多行直到下一个 [timestamp])
    """
    lines = text.split('\n')
    entries = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if not line.strip():
            i += 1
            continue
        if not _is_timestamp(line):
            i += 1
            continue

        # 时间戳行，检查下一行是否是 HEX
        if i + 1 < len(lines) and lines[i + 1].lstrip().startswith('← HEX:'):
            entry = [line]       # [0] timestamp
            entry.append(lines[i + 1])  # [1] ← HEX: ...
            i += 2
            # 收集 ASCII 部分（多行，直到下一个 [timestamp] 或 EOF）
            if i < len(lines) and lines[i].lstrip().startswith('← ASCII:'):
                ascii_lines = [lines[i]]
                i += 1
                while i < len(lines) and not _is_timestamp(lines[i]):
                    ascii_lines.append(lines[i])
                    i += 1
                entry.append(ascii_lines)  # [2] ASCII 多行列表
            entries.append(entry)
        else:
            entries.append([line])
            i += 1
    return entries


def extract_timestamp(line: str) -> str:
    """从时间戳行提取时间戳部分。"""
    m = re.match(r'^(\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}\])', line)
    return m.group(1) if m else line


def extract_hex_data(hex_line: str) -> str:
    """从 HEX 行提取纯 HEX 数据。"""
    m = re.match(r'\s*←\s*HEX:\s*(.*)', hex_line)
    return m.group(1).rstrip() if m else hex_line


def extract_ascii_lines(ascii_section: list[str]) -> list[str]:
    """从 ASCII 多行部分提取纯 ASCII 数据。第一行去掉 ← ASCII: 前缀。"""
    result = []
    for i, line in enumerate(ascii_section):
        if i == 0:
            m = re.match(r'\s*←\s*ASCII:\s*(.*)', line)
            result.append(m.group(1).rstrip() if m else line.rstrip())
        else:
            result.append(line.rstrip())
    return result


def convert_to_hex(entries: list[list[str]]) -> str:
    """转换为 HEX-only 格式。"""
    output = []
    for entry in entries:
        ts = extract_timestamp(entry[0])
        if len(entry) == 1:
            output.append(entry[0])
        elif len(entry) >= 2:
            hex_data = extract_hex_data(entry[1])
            output.append(f'{ts} {hex_data}')
    return '\n'.join(output) + '\n'


def convert_to_ascii(entries: list[list[str]]) -> str:
    """转换为 ASCII-only 格式。"""
    output = []
    for entry in entries:
        ts = extract_timestamp(entry[0])
        if len(entry) == 1:
            output.append(entry[0])
        elif len(entry) >= 3:
            ascii_lines = extract_ascii_lines(entry[2])
            output.append(f'{ts} {ascii_lines[0]}')
            for line in ascii_lines[1:]:
                output.append(line)
        elif len(entry) == 2:
            output.append(f'{ts}')
    return '\n'.join(output) + '\n'


def convert_file(input_path: str, fmt: str, output_dir: str | None = None):
    """转换单个文件。"""
    with open(input_path, 'r', encoding='utf-8') as f:
        text = f.read()

    entries = parse_entries(text)
    stem = Path(input_path).stem
    parent = output_dir or str(Path(input_path).parent)

    results = {}
    if fmt in ('hex', 'both'):
        hex_text = convert_to_hex(entries)
        hex_path = os.path.join(parent, f'{stem}_HEX.txt')
        with open(hex_path, 'w', encoding='utf-8') as f:
            f.write(hex_text)
        results['hex'] = hex_path

    if fmt in ('ascii', 'both'):
        ascii_text = convert_to_ascii(entries)
        ascii_path = os.path.join(parent, f'{stem}_ASCII.txt')
        with open(ascii_path, 'w', encoding='utf-8') as f:
            f.write(ascii_text)
        results['ascii'] = ascii_path

    return results


def main():
    parser = argparse.ArgumentParser(description='MIX 日志转换工具')
    parser.add_argument('input', nargs='?', help='输入文件路径')
    parser.add_argument('--format', '-f', choices=['hex', 'ascii', 'both'], default='both',
                        help='输出格式: hex / ascii / both (默认 both)')
    parser.add_argument('--output', '-o', help='输出目录 (默认与输入文件同目录)')
    parser.add_argument('--dir', '-d', help='批量处理目录下所有 log_*.txt 文件')
    args = parser.parse_args()

    if not args.input and not args.dir:
        parser.print_help()
        sys.exit(1)

    files = []
    if args.dir:
        files = sorted(Path(args.dir).glob('log_*.txt'))
        if not files:
            print(f'目录 {args.dir} 下没有找到 log_*.txt 文件')
            sys.exit(1)
    elif args.input:
        if not os.path.exists(args.input):
            print(f'文件不存在: {args.input}')
            sys.exit(1)
        files = [Path(args.input)]

    total = len(files)
    for i, fp in enumerate(files, 1):
        print(f'[{i}/{total}] {fp.name}', end='')
        results = convert_file(str(fp), args.format, args.output)
        parts = [f' -> {k}: {Path(v).name}' for k, v in results.items()]
        print(''.join(parts))

    print(f'完成，共处理 {total} 个文件')


if __name__ == '__main__':
    main()
