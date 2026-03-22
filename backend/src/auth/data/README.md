# Password denylist data

These files are used by `src/auth/password_policy.rs` to enforce ASVS 6.2 checks.

- `top3000-policy-passwords.txt`: Top 3000 high-frequency passwords with length >= 8.
- `breached-passwords.txt`: Known breached/common passwords with length >= 8.

Source dataset:
- `xato-net-10-million-passwords-100000.txt` from SecLists:
  https://github.com/danielmiessler/SecLists/tree/master/Passwords/Common-Credentials

Generation approach used in this repository:
- Top 3000 set: first 3000 entries from the source where `length >= 8`.
- Breached set: all entries from the source where `length >= 8`.
