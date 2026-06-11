#!/usr/bin/env python3
import pathlib, re, sys
root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else '.')
patterns = [
    re.compile(r'claude-sonnet', re.I),
    re.compile(r'VIL-native'),
    re.compile(r'rulebook\s+vil\.core', re.I),
    re.compile(r'refactor handlers/input\.rs'),
    re.compile(r'mutation gate'),
]
violations = []
for path in (root / 'vac-rs/crates/surfaces/vac-tui/src').rglob('*.rs'):
    rel = path.relative_to(root).as_posix()
    text = path.read_text(errors='ignore')
    for lineno, line in enumerate(text.splitlines(), 1):
        if 'assert!' in line or 'concat!(' in line:
            continue
        for pat in patterns:
            if pat.search(line):
                violations.append(f'{rel}:{lineno}: {line.strip()}')
if violations:
    print('TUI hardcoded mock-data violations:')
    print('\n'.join(violations))
    sys.exit(1)
print('PASS tui hardcoded mock-data gate')
