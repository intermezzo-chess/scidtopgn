---
default: patch
---

# Prevent name-frequency overflow from corrupting a database

Recount name references before changing a database, reject counts above
16,777,215, and discard a CLI import if it exceeds that limit. Updates that
keep the same names at the limit still succeed.
