lines = open('rullst/src/mail.rs').read().splitlines()
lines.insert(-5, "    }")
open('rullst/src/mail.rs', 'w').write('\n'.join(lines) + '\n')
