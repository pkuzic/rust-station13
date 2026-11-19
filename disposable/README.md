# Disposable

Everything in here is temporary and can be deleted at any time, whole or in part. Git
ignores all of it except this file.

- `builds/`: extra cargo target directories, for builds that must not share `target/`
  (`cargo test --target-dir disposable/builds/<name>`). Delete one once its job is done.
- `tmp/`: scratch scripts, logs, test maps, screenshots and drafts. Give a task its own
  subfolder (`tmp/<task>/`) and delete it when the task is merged.

The main build stays in `target/`, so incremental builds keep working.
