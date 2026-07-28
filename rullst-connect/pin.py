import os
import re
import urllib.request
import json

DIR = ".github/workflows"
actions_cache = {}

def get_sha(repo, ref):
    if (repo, ref) in actions_cache: return actions_cache[(repo, ref)]
    
    url = f"https://api.github.com/repos/{repo}/commits/{ref}"
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            sha = data['sha']
            actions_cache[(repo, ref)] = sha
            return sha
    except Exception as e:
        print(f"Error fetching {repo}@{ref}: {e}")
        return None

for filename in os.listdir(DIR):
    if not filename.endswith(".yml"): continue
    filepath = os.path.join(DIR, filename)
    with open(filepath, "r") as f:
        content = f.read()
    
    def replacer(match):
        full = match.group(0)
        action_path = match.group(1)
        ref = match.group(2)
        if len(ref) == 40: return full # already a sha
        
        # Extract owner/repo from action_path (e.g. github/codeql-action/init -> github/codeql-action)
        parts = action_path.split('/')
        repo = f"{parts[0]}/{parts[1]}"
        
        sha = get_sha(repo, ref)
        if sha:
            print(f"Replacing {action_path}@{ref} with {action_path}@{sha} in {filename}")
            return f"uses: {action_path}@{sha} # {ref}"
        return full

    new_content = re.sub(r'uses:\s*([a-zA-Z0-9_.-]+/[a-zA-Z0-9_/.-]+)@([^\s#]+)', replacer, content)
    with open(filepath, "w") as f:
        f.write(new_content)
