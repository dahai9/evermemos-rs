import os
import subprocess
import re
from difflib import SequenceMatcher

def get_git_log(n=100):
    # 获取最近 n 条提交的 hash 和 message
    try:
        out = subprocess.check_output(['git', 'log', '-n', str(n), '--pretty=format:%h|%s']).decode('utf-8')
        return [line.split('|', 1) for line in out.split('\n') if '|' in line]
    except Exception:
        return []

def slugify(text):
    """根据规范生成消息对应的 slug"""
    s = text.lower()
    # 替换冒号、空格等非字母数字字符为连字符
    s = re.sub(r'[^a-z0-9]+', '-', s)
    return s.strip('-')

def sync_solutions():
    # 脚本可能在 scripts/ 下运行，也可能在项目根目录运行
    # 这里我们通过 git rev-parse 获取项目根目录，以确保路径正确
    try:
        root = subprocess.check_output(['git', 'rev-parse', '--show-toplevel']).decode('utf-8').strip()
    except Exception:
        root = os.getcwd()

    sol_dir = os.path.join(root, "evermemos-rs/.solution")
    readme_path = os.path.join(sol_dir, "README.md")

    if not os.path.exists(sol_dir):
        print(f"❌ Error: {sol_dir} not found.")
        return

    log = get_git_log()
    if not log:
        print("❌ Error: Could not retrieve git log.")
        return

    # 读取 README 内容
    readme_content = ""
    if os.path.exists(readme_path):
        with open(readme_path, 'r', encoding='utf-8') as f:
            readme_content = f.read()

    modified_readme = False
    
    # 获取所有 .md 文档文件
    files = [f for f in os.listdir(sol_dir) if f.endswith(".md") and f != "README.md"]

    for filename in files:
        # 提取文件名里的 hash: abc1234-message.md
        match = re.match(r"^([a-f0-9]{7,})-(.*)\.md$", filename)
        if not match:
            continue

        file_hash, slug = match.groups()

        # 1. 首先尝试根据 Slug 修复 README 中可能存在的陈旧链接
        # 即：README 里引用了 old-hash-slug.md，但磁盘上已经是 new-hash-slug.md
        if readme_content:
            # 搜索所有符合 {7位或更多hash}-{slug}.md 格式但在 README 中且不是当前文件名的项
            # 这里我们查找 README 中包含相同 slug 的所有 .md 链接
            pattern = r"([a-f0-9]{7,})-" + re.escape(slug) + r"\.md"
            found_links = re.findall(pattern, readme_content)
            for old_link_hash in found_links:
                old_link_name = f"{old_link_hash}-{slug}.md"
                if old_link_name != filename:
                    print(f"🔧 Repairing stale README link: {old_link_name} -> {filename}")
                    readme_content = readme_content.replace(old_link_name, filename)
                    modified_readme = True

        # 2. 检查当前文件 Hash 是否还存在于当前的 log 中（处理 Rebase 导致的 Hash 变化）
        hash_exists = any(file_hash == h for h, _ in log)
        if hash_exists:
            continue

        # 如果 hash 不再存在，寻找语义最相似的提交消息进行同步
        best_match_hash = None
        best_match_slug = None
        max_ratio = 0.85 # 相似度阈值

        for new_hash, msg in log:
            current_slug = slugify(msg)
            # 这里比较文件名里的 slug 和当前 log 生成的 slug 的相似度
            ratio = SequenceMatcher(None, slug, current_slug).ratio()
            if ratio > max_ratio:
                max_ratio = ratio
                best_match_hash = new_hash
                best_match_slug = current_slug

        if best_match_hash:
            # 保持原有的 slug 风格或更新为最新的 slug（此处更新为最新以保持规范）
            new_filename = f"{best_match_hash}-{best_match_slug}.md"
            print(f"🔄 Detected Rebase: {filename} -> {new_filename}")
            
            old_path = os.path.join(sol_dir, filename)
            new_path = os.path.join(sol_dir, new_filename)
            
            try:
                # 执行 git mv
                subprocess.run(['git', 'mv', old_path, new_path], check=True)
                
                # 更新 README 里的索引字符串
                if readme_content:
                    # 替换所有出现的旧文件名
                    if filename in readme_content:
                        readme_content = readme_content.replace(filename, new_filename)
                        modified_readme = True
                        print(f"📝 Updated index for {filename} in README.md")
            except subprocess.CalledProcessError as e:
                print(f"⚠️ Failed to move {filename}: {e}")

    # 写回更新后的 README 并自动 commit
    if modified_readme:
        with open(readme_path, 'w', encoding='utf-8') as f:
            f.write(readme_content)
        
        print("🚀 Auto-committing rebase synchronization...")
        subprocess.run(['git', 'add', readme_path], check=True)
        # 自动提交同步结果，避免手动操作
        subprocess.run(['git', 'commit', '-m', "chore(evermemos-rs): sync .solution hashes after rebase [skip ci]"], check=True)
        print("✅ Done.")
    else:
        print("✨ Everything is up to date.")

if __name__ == "__main__":
    sync_solutions()