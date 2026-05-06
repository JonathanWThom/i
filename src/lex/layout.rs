use crate::token::TokenKind;

/// Tracks the state needed to decide whether a `\n` becomes a `Newline`
/// token, gets suppressed (line continuation), or is silently swallowed
/// (inside `(...)` / `[...]` parens).
///
/// Task 9 only handles `paren_depth` and `last_significant`. Task 10 will
/// add the indent stack to this struct for `Indent` / `Dedent` emission.
pub(super) struct Layout {
    pub paren_depth: u32,
    pub last_significant: Option<TokenKind>,
    /// Column widths of currently-open indented blocks. Always starts with
    /// 0 (top-level). Pushed when we see deeper indentation, popped when
    /// indentation drops back.
    pub indent_stack: Vec<u32>,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            paren_depth: 0,
            last_significant: None,
            indent_stack: vec![0],
        }
    }

    pub fn top(&self) -> u32 {
        *self.indent_stack.last().unwrap()
    }

    /// Update bookkeeping after a token has been pushed.
    pub fn note_emitted(&mut self, k: &TokenKind) {
        match k {
            TokenKind::LParen | TokenKind::LBracket => self.paren_depth += 1,
            TokenKind::RParen | TokenKind::RBracket => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
            }
            _ => {}
        }
        if !k.is_layout() {
            self.last_significant = Some(k.clone());
        }
    }

    /// True when the next `\n` should be silently consumed instead of
    /// becoming a `Newline` token.
    pub fn suppresses_newline(&self) -> bool {
        if self.paren_depth > 0 {
            return true;
        }
        // Binary operators and comma continue an expression onto the next
        // line. Binding operators (`:`, `=`, `->`) do NOT — they're often
        // followed by an indented block body, and the parser needs the
        // Newline + Indent pair to recognise that.
        matches!(
            self.last_significant,
            Some(
                TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Caret
                    | TokenKind::EqEq
                    | TokenKind::SlashEq
                    | TokenKind::Lt
                    | TokenKind::LtEq
                    | TokenKind::Gt
                    | TokenKind::GtEq
                    | TokenKind::PlusPlus
                    | TokenKind::Comma
            )
        )
    }
}
