import os
import re
import sys

def bump_version(new_version):
    files_to_update = [
        ("Cargo.toml", r'^version = ".*?"', f'version = "{new_version}"'),
]

    for filename, pattern, replacement in files_to_update:
        if not os.path.exists(filename):
            print(f"File not found: {filename}")
            continue

        with open(filename, 'r', encoding='utf-8') as f:
            content = f.read()

        new_content, num_subs = re.subn(pattern, replacement, content, flags=re.MULTILINE)

        if num_subs > 0:
            with open(filename, 'w', encoding='utf-8') as f:
                f.write(new_content)
            print(f"Updated {filename} to version {new_version}")
        else:
            print(f"No version matches found to update in {filename}")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python bump_version.py <new_version>")
        print("Example: python bump_version.py 0.5.0")
        sys.exit(1)
    
    bump_version(sys.argv[1])
