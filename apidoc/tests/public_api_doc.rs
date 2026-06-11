//! Public-API surface snapshots for the PARENT workspace (docs/public-api/).
//! Shared implementation + format docs: the `zenutils-apidoc` crate.
//!
//! `zentract-abi` is cdylib-only, so automatic discovery would skip it — the
//! crate list is explicit to keep all three surfaces (the `zentract-types` /
//! `zentract-abi` surfaces are the dlopen plugin compatibility contract).
#[test]
fn public_api_surface_docs_are_current() {
    zenutils_apidoc::ApiDoc::new()
        .workspace_dir("..")
        .crates(["zentract-types", "zentract-abi", "zentract-api"])
        .run();
}
