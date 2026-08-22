use super::*;
use pretty_assertions::assert_eq;

const PROFILE: RewriteProfile = RewriteProfile::new("SOURCE.md", &["source agent"])
    .with_case_sensitive_term_variants(&["Source"]);

#[test]
fn rewrites_terms_only_at_word_boundaries() {
    assert_eq!(
        PROFILE.rewrite("SOURCE.md Source source agent source_agent"),
        "AGENTS.md Codex Codex source_agent"
    );
}

#[test]
fn preserves_source_specific_filesystem_references() {
    assert_eq!(
        PROFILE.rewrite(
            "Read ./SOURCE.md, C:\\repo\\SOURCE.md, ~/.Source/config, and .Source before using Source."
        ),
        "Read ./SOURCE.md, C:\\repo\\SOURCE.md, ~/.Source/config, and .Source before using Codex."
    );
}

#[test]
fn preserves_source_specific_urls_and_dotted_identifiers() {
    assert_eq!(
        PROFILE.rewrite(
            "See https://Source.dev/docs, docs.Source.dev, and plugin.Source.config. Source."
        ),
        "See https://Source.dev/docs, docs.Source.dev, and plugin.Source.config. Codex."
    );
}

#[test]
fn rewrites_standalone_doc_name_but_not_path_qualified_doc_name() {
    assert_eq!(
        PROFILE.rewrite("SOURCE.md lives beside ../SOURCE.md and $HOME/SOURCE.md."),
        "AGENTS.md lives beside ../SOURCE.md and $HOME/SOURCE.md."
    );
}
