# Format + regenerate public-API snapshots
fmt:
    cargo fmt --all
    cargo test -p zentract-api --test public_api_doc

# Regenerate the public-API surface snapshots only
api-doc:
    cargo test -p zentract-api --test public_api_doc

# Verify the committed snapshots are current (what CI runs)
api-doc-check:
    ZEN_API_DOC=check cargo test -p zentract-api --test public_api_doc

# CI-exact clippy
clippy:
    cargo clippy --workspace --all-targets -- -D warnings
