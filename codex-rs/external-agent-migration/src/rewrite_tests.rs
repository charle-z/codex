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

#[test]
fn preserves_source_specific_terms_anywhere_inside_urls() {
    let input = concat!(
        "See https://example.test/?agent=claude#claude, ",
        "<https://example.test/search?q=Claude>, and claude://session?id=claude. ",
        "Use Claude."
    );
    let expected = concat!(
        "See https://example.test/?agent=claude#claude, ",
        "<https://example.test/search?q=Claude>, and claude://session?id=claude. ",
        "Use Codex."
    );

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}

#[test]
fn preserves_query_url_variants() {
    assert_eq!(
        CLAUDE_PROFILE.rewrite("https://example.test/?agent=claude"),
        "https://example.test/?agent=claude"
    );
    assert_eq!(
        CLAUDE_PROFILE.rewrite("https://[::1]/docs(foo)?agent=Claude"),
        "https://[::1]/docs(foo)?agent=Claude"
    );
}

#[test]
fn preserves_relative_uri_query_and_fragment_references() {
    let input = concat!(
        "?agent=claude foo#claude claude?agent=1 claude#section ",
        "//example.test/?agent=Claude#claude"
    );

    assert_eq!(CLAUDE_PROFILE.rewrite(input), input);
}

#[test]
fn preserves_quoted_paths_and_dotted_identifiers() {
    let input = r#"Read "/opt/Claude Code/config", 'C:\Program Files\Claude Code\config', \\server\Claude\config, and plugin.Claude.config. Use Claude Code."#;
    let expected = r#"Read "/opt/Claude Code/config", 'C:\Program Files\Claude Code\config', \\server\Claude\config, and plugin.Claude.config. Use Codex."#;

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}

#[test]
fn preserves_non_hierarchical_uri_references_without_hiding_prose_labels() {
    let input = concat!(
        "claude:session, ",
        "urn:claude:session, ",
        "mailto:claude@example.com. ",
        "Claude: use the UI."
    );
    let expected = concat!(
        "claude:session, ",
        "urn:claude:session, ",
        "mailto:claude@example.com. ",
        "Codex: use the UI."
    );

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}

#[test]
fn preserves_markdown_link_destinations_but_retargets_link_text_and_titles() {
    let input = concat!(
        "[relative](claude) [fragment](#claude) [query](?agent=claude) ",
        "[Claude](https://example.test/ \"Claude\")"
    );
    let expected = concat!(
        "[relative](claude) [fragment](#claude) [query](?agent=claude) ",
        "[Codex](https://example.test/ \"Codex\")"
    );

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}

#[test]
fn preserves_markdown_reference_destinations_but_retargets_titles() {
    let input = concat!(
        "[plain]: claude\n",
        "[angle]: <claude>\n",
        "[nested]: foo(claude)\n",
        "[titled]: https://example.test/ \"Claude\"\n"
    );
    let expected = concat!(
        "[plain]: claude\n",
        "[angle]: <claude>\n",
        "[nested]: foo(claude)\n",
        "[titled]: https://example.test/ \"Codex\"\n"
    );

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}

#[test]
fn preserves_escaped_markdown_inline_destinations() {
    let input =
        r#"[close](foo\)claude) [space](foo\ claude) [title](https://example.test/ "Claude")"#;
    let expected =
        r#"[close](foo\)claude) [space](foo\ claude) [title](https://example.test/ "Codex")"#;

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}

#[test]
fn markdown_like_prose_without_opening_bracket_still_retargets() {
    assert_eq!(
        CLAUDE_PROFILE.rewrite("foo](Claude) foo]: Claude"),
        "foo](Codex) foo]: Codex"
    );
}

#[test]
fn prose_punctuation_still_retargets() {
    let input = "Ask Claude? Ask Claude?\" Claude: use the UI.";
    let expected = "Ask Codex? Ask Codex?\" Codex: use the UI.";

    assert_eq!(CLAUDE_PROFILE.rewrite(input), expected);
}
