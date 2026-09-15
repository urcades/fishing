# Conformance vectors

Original vectors are copied from the accepted 0.1 demonstration corpus in
[fishing-examples](https://github.com/urcades/fishing-examples/tree/main/conformance/fixtures).
`trajectories.json` selects the spatial, bite, pressure and two-axis traces,
retaining all eight boundary cases. Numeric expectations are unchanged.
`provenance.json` describes the original full corpus; its counts refer to that corpus.
The harness explicitly migrates schema metadata, never numeric checkpoints.

`extensions.json` contains hand-derived 0.2 transition/RNG checkpoints. The core
tests additionally exercise target draw counts, extreme bounds and a complete
36,000-tick encounter. Full cross-language trajectories live in fishing-examples.
