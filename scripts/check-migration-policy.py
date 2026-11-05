#!/usr/bin/env python3
"""Review gate for new migrations; historical identities/content stay immutable.
SQL checks are deliberately conservative and supplement mandatory human DDL review.
"""
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
MIGRATIONS = ROOT / 'marreq-core/migrations'
BASELINE = ROOT / 'tools/marreq-migrations/released-migrations.json'


def check(migrations=MIGRATIONS, baseline=BASELINE):
    import hashlib
    released = json.loads(baseline.read_text())
    for name, checksum in released.items():
        path = migrations / name
        assert path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest() == checksum, f'released migration modified/missing: {name}'
    seen = set()
    for directory in sorted(migrations.iterdir()):
        if not directory.is_dir():
            continue
        version = directory.name.split('_', 1)[0].replace('-', '')
        assert version.isdigit() and version not in seen, f'invalid/duplicate identity: {directory.name}'
        seen.add(version)
        if f'{directory.name}/up.sql' in released:
            continue
        assert version > max(p.split('_', 1)[0].replace('-', '') for p in released), 'new migrations must follow released history'
        policy = json.loads((directory / 'review.json').read_text())
        assert policy['phase'] == 'expand', 'contract/data changes require a separately reviewed release gate; not allowed by the expansion gate'
        for field in ['compatibility_window', 'lock_risk', 'rollback_limit', 'validation']:
            assert isinstance(policy[field], str) and policy[field].strip(), f'missing review field: {field}'
        sql = (directory / 'up.sql').read_text()
        sql = re.sub(r'/\*.*?\*/|--[^\n]*', '', sql, flags=re.S)
        assert not re.search(r'\b(DROP|TRUNCATE|DELETE|UPDATE|RENAME)\b|\bSET\s+NOT\s+NULL\b', sql, re.I), 'potentially breaking/data-changing SQL requires separate review'
        assert not re.search(r'\bADD\b[^;]*\bNOT\s+NULL\b', sql, re.I), 'new NOT NULL columns/constraints require transition review'
        assert (directory / 'down.sql').is_file(), 'Diesel down.sql required; not an automatic recovery operation'
    print(f'Migration policy passed: {len(seen)} identities; released files unchanged')


if __name__ == '__main__':
    check()
