# Disposable

Everything in here can be rebuilt or thrown away, and can be deleted at any time, whole or
in part. Git ignores all of it except this file.

- `target/`: the main build, with its debug info (`.pdb` files). `.cargo/config.toml` points
  cargo here. Delete stale files from it as described in `AGENTS.md`.
- `builds/`: extra target directories, for builds that must not share the main one
  (`cargo test --target-dir disposable/builds/<name>`). Delete one once its job is done.
- `tmp/`: scratch scripts, logs, test maps, screenshots and drafts. Give a task its own
  subfolder (`tmp/<task>/`) and delete it when the task is merged.
