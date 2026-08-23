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
#  is deliberately absent: preparing a statement takes no lock, so a
#  before an  is still a write-first transaction. Counting it
# as a read misclassified sites and would have caused conversions that are not
# needed -- which is the mechanical replacement this whole exercise refuses.
READ = re.compile(r'\.(query_row|query_map|query_and_then|query\()')
WRITE = re.compile(r'\.(execute|execute_batch|insert)\b')
# `query_row` around an UPDATE/INSERT/DELETE ... RETURNING is a *write*. Reading it as a
# read classified a single-statement compare-and-swap as a multi-read, which would have
# turned a correct write-first transaction into one that never writes.
RETURNING_WRITE = re.compile(r'\b(UPDATE|INSERT|DELETE)\b', re.IGNORECASE)
# Deliberately narrow. An earlier version matched `keyring` anywhere, which fired
# on a *string literal* naming a keyring entry -- a reference being written to a
# column, not a call. A flag that fires on data rather than on a call is a flag a
# reviewer learns to ignore.
EXTERNAL = re.compile(
    r'(std::fs::|tokio::fs::|reqwest::|keyring::|Command::new|\.send\(\)\.|'
    r'fs::read_to_string|fs::write|create_dir_all|remove_file|remove_dir_all|'
    r'OsCredentialStore::|\.await)'
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
        test_mod_line = next(
            (n for n, l in enumerate(lines)
             if l.strip() == '#[cfg(test)]'
             and n + 1 < len(lines) and 'mod ' in lines[n + 1]),
            None,
        )
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
                def mutates(position):
                    return bool(RETURNING_WRITE.search(
                        '\n'.join(body[position:position + 12])))

                # A read is only a read if the statement it runs is not a mutation.
                first_read = next(
                    (n for n, l in enumerate(body)
                     if READ.search(l) and not mutates(n)),
                    None,
                )
                first_write = next(
                    (n for n, l in enumerate(body)
                     if WRITE.search(l) or (READ.search(l) and mutates(n))),
                    None,
                )
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
                    # A production file can hold an inline `#[cfg(test)] mod tests`.
                    # Judging by filename alone reported a test's `unwrap()` as a
                    # production defect.
                    'test': (
                        '_tests.rs' in path
                        or '/tests.rs' in path
                        # A whole directory of tests carries no marker either.
                        or '/tests/' in path
                        or (test_mod_line is not None and index > test_mod_line)
                    ),
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
