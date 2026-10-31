# Weather Glass

Rain drops, running streaks, snow and frost on glass, drawn at 64 pixels per block, for
Horizon's own shader packs (docs/shaders.md). It reads the weather where you stand: drops
and streaks while it rains, frost creeping in from the edges while it snows, and
`wetness` lets the glass dry slowly after the rain stops. Only glass under the open sky
gets wet. Options: Drops, Streaks, Snow & frost, Density.

Glass is picked by its sprites (`[[tags]]` in `horizon-shaders.toml`): plain, tinted and
stained glass blocks and panes' faces.
