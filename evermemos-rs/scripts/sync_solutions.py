import os
import subprocess
import re
from difflib import SequenceMatcher
    
def get_git_log():
    # 获取最近 50 条提交的 hash 和 message
    out = subprocess.check_output(['git', 'log', '-n', '50', '--pretty=format:%h|%s']).decode()
    return [line.split('|') for line in out.split('\n')]

def sync_solutions():
    log = get_git_log()
    sol_dir = "../.solution"

    for filename in os.listdir(sol_dir):
        if not filename.endswith(".md") or filename == "README.md":
            continue

        # 提取文件名里的 hash: abc1234-message.md
        match = re.match(r"^([a-f0-9]{7})-(.*)\.md$", filename)
        if not match: continue

        old_hash, slug = match.groups()

        # 检查 old_hash 是否还存在
        hash_exists = any(old_hash == item[0] for item in log)
        if hash_exists: continue

        # 如果不存在，寻找最相似的提交消息
        best_match = None
        max_ratio = 0.8 # 阈值

        for new_hash, msg in log:
            # 简单清理 msg 转为 slug 比较
            current_slug = msg.lower().replace(" ", "-").replace(":", "-")
            ratio = SequenceMatcher(None, slug, current_slug).ratio()
            if ratio > max_ratio:
                max_ratio = ratio
                best_match = new_hash

        if best_match:
            new_filename = f"{best_match}-{slug}.md"
            print(f"🔄 Syncing: {old_hash} -> {best_match} ({filename})")
            subprocess.run(['git', 'mv', os.path.join(sol_dir, filename), os.path.join(sol_dir, new_filename)])
            # 接下来可以在这里继续写更新 README.md 的逻辑...

if __name__ == "__main__":
    sync_solutions()