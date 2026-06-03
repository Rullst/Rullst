import os
import re

BLUEPRINTS_DIR = r"c:\Users\venelouis\Desktop\REPOS\Rullst\cargo-rullst\src\blueprints"
FILES = ["portfolio.rs", "lms.rs", "saas.rs", "blog.rs"]

for filename in FILES:
    filepath = os.path.join(BLUEPRINTS_DIR, filename)
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    # Find the main_rs format string
    pattern = r'(let main_rs = format!\(r##"(.*?)##,\s*project_name_safe = project_name_safe\);)'
    
    def replacer(match):
        full_match = match.group(0)
        inner_content = match.group(2)
        
        # We need to escape { and } EXCEPT around project_name_safe
        # First, temporarily replace {project_name_safe} with a unique token
        temp_content = inner_content.replace("{project_name_safe}", "___PROJECT_NAME___")
        
        # Now escape all { and }
        temp_content = temp_content.replace("{", "{{").replace("}", "}}")
        
        # Now restore the {project_name_safe}
        temp_content = temp_content.replace("___PROJECT_NAME___", "{project_name_safe}")
        
        return f'let main_rs = format!(r##"{temp_content}##, project_name_safe = project_name_safe);'

    new_content = re.sub(pattern, replacer, content, flags=re.DOTALL)

    with open(filepath, "w", encoding="utf-8") as f:
        f.write(new_content)

    print(f"Fixed {filename}")
