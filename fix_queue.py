with open('rullst/src/queue.rs', 'r') as f:
    content = f.read()

import re

# We have two test_sqlite_queue_list_all_jobs.
# One starts around 805, the other at 904.

pattern = re.compile(r'    #\[tokio::test\]\n    async fn test_sqlite_queue_list_all_jobs\(\) \{.*?\n    \}\n', re.DOTALL)
matches = list(pattern.finditer(content))
if len(matches) > 1:
    # Remove the second one
    match = matches[1]
    new_content = content[:match.start()] + content[match.end():]
    with open('rullst/src/queue.rs', 'w') as f:
        f.write(new_content)
