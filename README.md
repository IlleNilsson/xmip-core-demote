# xmip-core-demote

Demotion: writing Message Context values back out — into a structured
Message, or into an executable artifact's configuration. Which surface a value
is written onto is the `DemotionTarget`, and it decides the cost: writing into
the payload changes the Stream and so produces a new one; writing into a
transport property changes only the envelope.

Demotion is not promotion in reverse for free: it is the one direction that
can create a Stream. It does not decide what to write; a Path and a content
selector do.

`doc/architecture/runtime-model.md` section 9 governs it, and the selector
language is `module/capability/promote/doc/content-selector.md`;
`architecture.toml` carries the maturity.
