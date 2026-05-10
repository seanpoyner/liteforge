#!/usr/bin/env python3
"""Hello world test for liteforge."""

from liteforge import ForgeClient

client = ForgeClient()
response = client.chat([{"role": "user", "content": "Say hello in one sentence."}])

print(response["choices"][0]["message"]["content"])
