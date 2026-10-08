use bitflags::bitflags;

bitflags! {
    ///The parsing context that is in effect while a piece of the token stream is
    ///being parsed. Flags are threaded explicitly through the parse methods
    ///instead of living on the [`Parser`](crate::Parser), so every place that
    ///changes how a construct is read has to say so at the call site.
    ///
    ///The derived [`Default`] is [`ParserFlags::empty`], i.e. "no context".
    ///`REQUIRE_SEMICOLON` and `COMPONENT_EXPR` are the two flags that change how
    ///expressions and statements are read, and both are requested explicitly by
    ///the construct that needs them (a block, or a `let`/`return` statement).
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ParserFlags: u64 {
        ///The current statement is terminated by a `;`. Checked by
        ///[`Parser::finish_current_parse`](crate::Parser::finish_current_parse).
        const REQUIRE_SEMICOLON = 1 << 0;
        ///Only the signature of the declaration is present, without a body. Set
        ///for everything inside an `extern { .. }` block and for interface
        ///method declarations.
        const ONLY_SIGNATURES = 1 << 1;
        ///A bare `Name { .. }` following an identifier is a component literal
        ///rather than a plain identifier followed by a block. Enabled inside
        ///function/block bodies, disabled while parsing `if` conditions so that
        ///`if x matches Variant { field: 1 }` keeps its pattern.
        const COMPONENT_EXPR = 1 << 2;
    }
}
