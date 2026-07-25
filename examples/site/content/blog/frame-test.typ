= Frame Test

This post tests `html.frame()` rendering in content entries. Frames
use Typst's PDF / SVG layout engine.

== Typst Frame

Below is a laid-out box rendered with `html.frame()`:

#html.frame[
  *This text is emphasised inside a laid-out frame.*
  #lorem(6)
]

== Colored Frame

Below is a frame with an inline SVG background:

#html.frame[
  #block(
    fill: rgb("#d73a49"),
    inset: 8pt,
  )[#text(fill: white)[A colored frame rendered as SVG.]]
]
