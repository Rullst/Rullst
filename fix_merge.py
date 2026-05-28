import re

with open('rullst/src/mail.rs', 'r') as f:
    content = f.read()

# Pattern to find git merge conflict markers
pattern = re.compile(
    r'<<<<<<< HEAD\n'
    r'(.*?)\n'
    r'=======\n'
    r'(.*?)\n'
    r'>>>>>>> origin/main\n',
    re.DOTALL
)

def replace_func(match):
    head_content = match.group(1)
    main_content = match.group(2)

    # We want both the tests from HEAD and the tests from main.
    # We can just keep both blocks since they are at the end of the module.

    # We strip any trailing braces and whitespace to merge cleanly
    # Actually wait, let's just see what's in head and main.
    return f"{head_content}\n{main_content}"

new_content = pattern.sub(replace_func, content)

with open('rullst/src/mail.rs', 'w') as f:
    f.write(new_content)
