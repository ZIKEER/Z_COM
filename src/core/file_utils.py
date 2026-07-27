import json
import os
import tempfile


def atomic_write_json(file_path, data):
    """将 JSON 原子写入目标文件，失败时保留原文件。"""
    target_path = os.path.abspath(file_path)
    target_dir = os.path.dirname(target_path)
    os.makedirs(target_dir, exist_ok=True)
    fd, temp_path = tempfile.mkstemp(
        prefix=f".{os.path.basename(target_path)}.", suffix=".tmp", dir=target_dir,
    )
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as file_obj:
            json.dump(data, file_obj, ensure_ascii=False, indent=2)
            file_obj.flush()
            os.fsync(file_obj.fileno())
        os.replace(temp_path, target_path)
    except Exception:
        try:
            os.unlink(temp_path)
        except OSError:
            pass
        raise
