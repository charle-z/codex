from pathlib import Path

path = Path('.github/workflows/validate-39951-fork-source-v4.yml')
text = path.read_text()
old = '''          cargo clippy -p codex-hooks --all-targets -- -D warnings
          cargo clippy -p codex-core --tests -- -D warnings
'''
new = '''          # Current upstream has one unrelated test-only unused import that
          # just fix removes. Keep that baseline fix only while clippy runs.
          just fix -p codex-core
          cargo clippy -p codex-hooks --all-targets -- -D warnings
          cargo clippy -p codex-core --tests -- -D warnings
          cd ..
          git restore --source=HEAD -- codex-rs/core/tests/suite/openai_file_mcp.rs
          git diff --check
          git diff --exit-code
'''
count = text.count(old)
if count != 1:
    raise SystemExit(f'expected one clippy block, found {count}')
path.write_text(text.replace(old, new, 1))
