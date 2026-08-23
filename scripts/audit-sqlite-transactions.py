"""First-pass classifier for every SQLite transaction site.

Reports, per site, what happens between the transaction opening and its commit:
whether a read precedes the first write, whether external I/O appears, and how
long the body is. The output is a starting point for review, not a verdict --
every site still gets read.
"""
import io
import os
import re

ROOT = 'src-tauri/src'
OPENERS = [
    ('.transaction()', 'deferred'),
    ('unchecked_transaction()', 'deferred'),
    ('transaction_with_behavior(TransactionBehavior::Immediate)', 'immediate'),
    ('begin_write_transaction(', 'immediate-helper'),
    ('begin_read_transaction(', 'deferred-helper'),
]
READ = re.compile(r'\.(query_row|query_map|prepare|query_and_then|query\()')
WRITE = re.compile(r'\.(execute|execute_batch|insert)\b')
EXTERNAL = re.compile(
    r'(std::fs|tokio::fs|reqwest|keyring|Command::new|\.send\(\)|read_to_string|'
    r'write_all|create_dir|remove_file|remove_dir|OsCredentialStore|\.await)'
)
COMPUTE = re.compile(r'(Sha256|sha256|serde_json::to_string|serde_json::from_str|\.sort|collect::<)')

rows = []
for base, _, names in os.walk(ROOT):
    for name in names:
        if not name.endswith('.rs'):
            continue
        path = os.path.join(base, name).replace('\\', '/')
        source = io.open(path, encoding='utf-8').read()
        lines = source.splitlines()
        for index, line in enumerate(lines):
            for token, kind in OPENERS:
                if token not in line:
                    continue
                # The transaction body: until the matching commit, or 80 lines.
                body = []
                for follow in lines[index + 1:index + 81]:
                    body.append(follow)
                    if '.commit()' in follow:
                        break
                text = '\n'.join(body)
                first_read = next(
                    (n for n, l in enumerate(body) if READ.search(l)), None)
                first_write = next(
                    (n for n, l in enumerate(body) if WRITE.search(l)), None)
                if first_read is None and first_write is None:
                    shape = 'empty-or-unclear'
                elif first_write is None:
                    shape = 'multi-read'
                elif first_read is None:
                    shape = 'write-first'
                elif first_read < first_write:
                    shape = 'READ-THEN-WRITE'
                else:
                    shape = 'write-then-read'
                rows.append({
                    'path': path,
                    'line': index + 1,
                    'kind': kind,
                    'shape': shape,
                    'body_lines': len(body),
                    'external': bool(EXTERNAL.search(text)),
                    'compute': bool(COMPUTE.search(text)),
                    'test': '_tests.rs' in path or '/tests.rs' in path,
                })

production = [r for r in rows if not r['test']]
print(f'sites: {len(rows)} total, {len(production)} production\n')

for shape in ['READ-THEN-WRITE', 'multi-read', 'write-first', 'write-then-read',
              'empty-or-unclear']:
    group = [r for r in production if r['shape'] == shape]
    print(f'--- {shape}: {len(group)} ---')
    for r in group:
        flags = []
        if r['external']:
            flags.append('EXTERNAL-IO')
        if r['compute']:
            flags.append('compute')
        if r['body_lines'] > 40:
            flags.append(f"long({r['body_lines']})")
        suffix = ('  [' + ' '.join(flags) + ']') if flags else ''
        print(f"  {r['kind']:18} {r['path']}:{r['line']}{suffix}")
    print()
