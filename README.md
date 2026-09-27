# xmip-core-demote

Demotion: writing Message Context values back out — into a structured
Message, or into an executable artifact's configuration. Which surface a value
is written onto is the `DemotionTarget`, and it decides the cost: writing into
the payload changes the Stream and so produces a new one; writing into a
transport property changes only the envelope. A `Demotion` into the payload
is compiled once through the path engine (`Demotion::compile`, `None` for
every other target), and `apply_to_structure` writes every compiled one into
one `path::Rewriting`, which opens the Stream once and writes the new one
once.

Demotion is not promotion in reverse for free: it is the one direction that
can create a Stream. It does not decide what to write; a Path and a content
selector do.

`doc/architecture/runtime-model.md` section 9 governs it, and the selector
language is `module/core/capability/promote/doc/content-selector.md`;
`architecture.toml` carries the maturity.
