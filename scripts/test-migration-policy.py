#!/usr/bin/env python3
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('policy', pathlib.Path(__file__).with_name('check-migration-policy.py'))
policy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(policy)


class PolicyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = pathlib.Path(self.temp.name)
        old = self.root / '2026-01-01-000001_released'
        old.mkdir()
        (old / 'up.sql').write_text('CREATE TABLE example(id int);')
        self.baseline = self.root / 'released.json'
        self.baseline.write_text(json.dumps({str((old/'up.sql').relative_to(self.root)): hashlib.sha256((old/'up.sql').read_bytes()).hexdigest()}))
        self.new = self.root / '2026-11-05-000001_expand'
        self.new.mkdir()
        (self.new / 'up.sql').write_text('ALTER TABLE example ADD COLUMN optional text;')
        (self.new / 'down.sql').write_text('ALTER TABLE example DROP COLUMN optional;')
        self.review = dict(phase='expand', compatibility_window='retained/candidate', lock_risk='5s', rollback_limit='column retained', validation='old/new smoke')
        self.write_review()

    def write_review(self):
        (self.new/'review.json').write_text(json.dumps(self.review))

    def check(self):
        policy.check(self.root, self.baseline)

    def test_nullable_expansion(self):
        self.check()

    def test_released_content_is_frozen(self):
        (self.root/'2026-01-01-000001_released/up.sql').write_text('SELECT 1;')
        with self.assertRaises(AssertionError): self.check()

    def test_breaking_or_data_changing_sql(self):
        for sql in ['ALTER TABLE example DROP COLUMN id;', 'UPDATE example SET id=0;', 'ALTER TABLE example ADD COLUMN bad text NOT NULL;']:
            with self.subTest(sql=sql):
                (self.new/'up.sql').write_text(sql)
                with self.assertRaises(AssertionError): self.check()

    def test_contract_requires_separate_review(self):
        self.review['phase']='contract'; self.write_review()
        with self.assertRaises(AssertionError): self.check()

    def test_missing_validation(self):
        del self.review['validation']; self.write_review()
        with self.assertRaises(KeyError): self.check()

    def test_duplicate_version(self):
        (self.root/'2026-11-05-000001_zduplicate').mkdir()
        with self.assertRaises(AssertionError): self.check()


if __name__ == '__main__':
    unittest.main()
