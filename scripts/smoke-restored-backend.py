#!/usr/bin/env python3
"""Authenticated read/download from the disposable restore fixture only."""
import http.cookiejar
import json
import sys
import urllib.request

base = sys.argv[1]
assert base.startswith('http://127.0.0.1:')
client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
csrf = json.load(client.open(base + '/api/auth/csrf'))['csrf_token']
request = urllib.request.Request(base + '/api/auth/login',
    data=json.dumps(dict(username='alice', password='ChangeMe123!')).encode(),
    headers={'Content-Type': 'application/json', 'X-CSRF-Token': csrf, 'Origin': base})
client.open(request).read()
assert json.load(client.open(base + '/api/auth/me'))['username'] == 'alice'
assert client.open(base + '/api/projects/1/attachments/1/download').read() == b'migration recovery fixture'
print('Restored backend authenticated read and attachment download passed')
