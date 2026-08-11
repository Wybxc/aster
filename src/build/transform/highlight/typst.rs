use ::typst::ecow::EcoVec;
use ::typst::syntax::{LinkedNode, SyntaxKind, SyntaxNode, Tag, parse_code, parse_math};

use super::HighlightToken;

#[comemo::memoize]
pub fn highlight(code: &str, lang: &str) -> Option<EcoVec<HighlightToken>> {
    let root: SyntaxNode = match lang {
        "typ" | "typst" => ::typst::syntax::parse(code),
        "typc" => parse_code(code),
        "typm" => parse_math(code),
        _ => return None,
    };

    let mut tokens = EcoVec::new();
    walk(code, &LinkedNode::new(&root), "", &mut tokens);
    Some(tokens)
}

fn walk(
    code: &str,
    node: &LinkedNode<'_>,
    inherited_scope: &'static str,
    tokens: &mut EcoVec<HighlightToken>,
) {
    let scope = node_scope(node, inherited_scope);
    if node.children().len() == 0 {
        let text = &code[node.range()];
        if node.kind() == SyntaxKind::Label
            && let Some(label) = text
                .strip_prefix('<')
                .and_then(|text| text.strip_suffix('>'))
        {
            push_token(tokens, "operator", "<");
            push_token(tokens, "markup.link.label", label);
            push_token(tokens, "operator", ">");
        } else {
            push_token(tokens, scope, text);
        }
        return;
    }

    for child in node.children() {
        walk(code, &child, scope, tokens);
    }
}

fn node_scope(node: &LinkedNode<'_>, inherited: &'static str) -> &'static str {
    match node.kind() {
        SyntaxKind::Heading => heading_scope(node),
        SyntaxKind::Equation | SyntaxKind::Math => "markup.math",
        SyntaxKind::MathText if is_math_number(node) => "number",
        SyntaxKind::Hash => "punctuation.special",
        SyntaxKind::RawLang => "label",
        SyntaxKind::Text | SyntaxKind::RawTrimmed if is_raw_block_body(node) => "markup.raw.block",
        SyntaxKind::LeftBrace
        | SyntaxKind::RightBrace
        | SyntaxKind::LeftBracket
        | SyntaxKind::RightBracket
        | SyntaxKind::LeftParen
        | SyntaxKind::RightParen => "punctuation.bracket",
        SyntaxKind::Comma | SyntaxKind::Semicolon | SyntaxKind::Colon => "punctuation.delimiter",
        SyntaxKind::ListMarker | SyntaxKind::EnumMarker | SyntaxKind::TermMarker => "operator",
        SyntaxKind::Plus | SyntaxKind::Minus | SyntaxKind::Star if in_math(node) => inherited,
        SyntaxKind::Star => "operator",
        SyntaxKind::Dollar
        | SyntaxKind::Hat
        | SyntaxKind::Underscore
        | SyntaxKind::Root
        | SyntaxKind::MathAlignPoint
        | SyntaxKind::MathPrimes
        | SyntaxKind::MathShorthand
        | SyntaxKind::Arrow
        | SyntaxKind::Dot => inherited,
        SyntaxKind::Not | SyntaxKind::And | SyntaxKind::Or => "operator",
        SyntaxKind::Import | SyntaxKind::Include => "keyword.import",
        SyntaxKind::If | SyntaxKind::Else => "keyword.conditional",
        SyntaxKind::For | SyntaxKind::While | SyntaxKind::Break | SyntaxKind::Continue => {
            "keyword.repeat"
        }
        SyntaxKind::In if has_ancestor(node, SyntaxKind::ForLoop) => "keyword.repeat",
        SyntaxKind::In => "operator",
        SyntaxKind::Let | SyntaxKind::Set | SyntaxKind::Show => "keyword",
        SyntaxKind::Context
        | SyntaxKind::Return
        | SyntaxKind::None
        | SyntaxKind::Auto
        | SyntaxKind::As => inherited,
        SyntaxKind::Bool => "boolean",
        SyntaxKind::Ident if node.parent_kind() == Some(SyntaxKind::Named) => "variable.member",
        SyntaxKind::Ident | SyntaxKind::MathIdent => match ::typst::syntax::highlight(node) {
            Some(Tag::Function) => "function.call",
            _ => "constant",
        },
        _ => ::typst::syntax::highlight(node)
            .map(|tag| lumis_scope(tag, inherited))
            .unwrap_or(inherited),
    }
}

fn lumis_scope(tag: Tag, inherited: &'static str) -> &'static str {
    match tag {
        Tag::Comment => "comment",
        Tag::Punctuation => "punctuation.delimiter",
        Tag::Strong => "markup.strong",
        Tag::Emph => "markup.italic",
        Tag::Raw => "markup.raw",
        Tag::Label => "markup.link.label",
        Tag::Ref => "markup.link",
        Tag::Heading => "markup.heading",
        Tag::MathGroupingParens => "punctuation.bracket",
        Tag::Keyword => "keyword",
        Tag::MathOperator | Tag::Operator => "operator",
        Tag::Number => "number",
        Tag::String => "string",
        Tag::Function => "function.call",
        Tag::Interpolated => "constant",
        Tag::Escape
        | Tag::Link
        | Tag::ListMarker
        | Tag::ListTerm
        | Tag::MathDelimiter
        | Tag::Error => inherited,
    }
}

fn heading_scope(node: &LinkedNode<'_>) -> &'static str {
    let depth = node
        .children()
        .find(|child| child.kind() == SyntaxKind::HeadingMarker)
        .map_or(1, |marker| marker.len());
    match depth {
        1 => "markup.heading.1",
        2 => "markup.heading.2",
        3 => "markup.heading.3",
        4 => "markup.heading.4",
        5 => "markup.heading.5",
        _ => "markup.heading.6",
    }
}

fn is_raw_block_body(node: &LinkedNode<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == SyntaxKind::Raw
            && parent
                .children()
                .next()
                .is_some_and(|delimiter| delimiter.len() >= 3)
    })
}

fn has_ancestor(node: &LinkedNode<'_>, kind: SyntaxKind) -> bool {
    let mut ancestor = node.parent();
    while let Some(node) = ancestor {
        if node.kind() == kind {
            return true;
        }
        ancestor = node.parent();
    }
    false
}

fn in_math(node: &LinkedNode<'_>) -> bool {
    has_ancestor(node, SyntaxKind::Math) || has_ancestor(node, SyntaxKind::Equation)
}

fn is_math_number(node: &LinkedNode<'_>) -> bool {
    let text = node.get().leaf_text();
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    match text.split_once('.') {
        Some((integer, fractional)) => digits(integer) && digits(fractional),
        None => digits(text),
    }
}

fn push_token(tokens: &mut EcoVec<HighlightToken>, scope: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some((language, previous_scope, previous_text)) = tokens.make_mut().last_mut()
        && language == "typst"
        && previous_scope == scope
    {
        previous_text.push_str(text);
    } else {
        tokens.push(("typst".into(), scope.into(), text.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighted_text(tokens: &[HighlightToken]) -> String {
        tokens.iter().map(|(_, _, text)| text.as_str()).collect()
    }

    fn assert_token(tokens: &[HighlightToken], scope: &str, text: &str) {
        assert!(
            tokens.iter().any(|(language, actual_scope, actual_text)| {
                language == "typst" && actual_scope == scope && actual_text == text
            }),
            "missing ({scope:?}, {text:?}) in {tokens:#?}",
        );
    }

    #[test]
    fn preserves_source_text_in_all_modes() {
        for (language, source) in [
            ("typst", "= Heading\n#let x = 1\n"),
            ("typc", "let x = 1\nif x > 0 { x }"),
            ("typm", "x^2 + y/2"),
        ] {
            assert_eq!(
                highlighted_text(&highlight(source, language).unwrap()),
                source
            );
        }
    }

    #[test]
    fn maps_native_syntax_to_lumis_scopes() {
        let markup = concat!(
            "== Heading\n",
            "*strong* _emph_ `raw` <label> @ref\n",
            "$ x^2 + y $\n",
            "#let f(x) = x\n",
            "#import \"x.typ\"\n",
            "#if true [yes] else [no]\n",
            "#for x in xs { break }\n",
            "#let d = (field: true)\n",
            "```rust\nlet x = 1;\n```\n",
        );
        let tokens = highlight(markup, "typst").unwrap();
        for (scope, text) in [
            ("markup.heading.2", "== Heading"),
            ("markup.strong", "strong"),
            ("markup.italic", "_emph_"),
            ("markup.raw", "`raw`"),
            ("markup.link.label", "label"),
            ("markup.link", "@ref"),
            ("punctuation.special", "#"),
            ("function.call", "f"),
            ("keyword.import", "import"),
            ("keyword.conditional", "if"),
            ("keyword.repeat", "for"),
            ("boolean", "true"),
            ("variable.member", "field"),
            ("label", "rust"),
            ("markup.raw.block", "\nlet x = 1;\n"),
        ] {
            assert_token(&tokens, scope, text);
        }

        let code = "let answer = 42\nif answer > 0 { answer } else { 0 }";
        let tokens = highlight(code, "typc").unwrap();
        assert_token(&tokens, "keyword", "let");
        assert_token(&tokens, "keyword.conditional", "if");
        assert_token(&tokens, "keyword.conditional", "else");

        let tokens = highlight("x^2 + y/2", "typm").unwrap();
        assert_token(&tokens, "number", "2");
        assert_token(&tokens, "operator", "/");
    }
}
