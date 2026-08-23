use super::*;
use pretty_assertions::assert_eq;

const PROFILE: RewriteProfile = RewriteProfile::new("SOURCE.md", &["source agent"])
    .with_case_sensitive_term_variants(&["Source"]);

const CLAUDE_PROFILE: RewriteProfile = RewriteProfile::new(
    "CLAUDE.md",
    &[
        "claude code",
        "claude-code",
        "claude_code",
        "claudecode",
        "claude",
    ],
);

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

#[test]
fn preserves_claude_skill_paths_while_retargeting_prose() {
    let input = concat!(
        "Session transcripts live in ~/.claude/projects/<slug>/.\n",
        "See also .claude/commands/example.md and C:\\Users\\me\\.claude\\plans\\today.md.\n",
        "Use Claude Code UI and read CLAUDE.md."
    );
    let expected = concat!(
        "Session transcripts live in ~/.claude/projects/<slug>/.\n",
        "See also .claude/commands/example.md and C:\\Users\\me\\.claude\\plans\\today.md.\n",
        "Use Codex UI and read AGENTS.md."
    );

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}
