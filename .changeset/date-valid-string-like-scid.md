---
default: patch
---

# Accept the same short dates as SCID in `date_valid_string`, and reject numbers too large for a `u32`

`date_valid_string` now follows SCID's `date_ValidString` for dates with fewer than three parts, so `2022`, `2022.12`, `2022.`, and an empty string are valid. It previously required three parts.

A part too large for a `u32` is now invalid, so `99999999999.01.01` is rejected. It was previously valid. SCID's answer for such numbers depends on the platform: on 64-bit macOS and Linux it accepts a number below 2^64 that is in range once cut to 32 bits, such as `4294967301.01.01`, and `date_valid_string` doesn't.
