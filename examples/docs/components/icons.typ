#let _svg(body, view-box: "0 0 24 24", fill: "none") = html.elem("svg", attrs: (
  class: "sl-icon",
  xmlns: "http://www.w3.org/2000/svg",
  viewBox: view-box,
  fill: fill,
  stroke: "currentColor",
  "stroke-width": "2",
  "stroke-linecap": "round",
  "stroke-linejoin": "round",
  "aria-hidden": "true",
))[#body]

#let menu-icon = _svg[
  #html.elem("path", attrs: (d: "M4 6h16M4 12h16M4 18h16"))
]

#let close-icon = _svg[
  #html.elem("path", attrs: (d: "M18 6 6 18M6 6l12 12"))
]

#let search-icon = _svg[
  #html.elem("circle", attrs: (cx: "11", cy: "11", r: "8"))
  #html.elem("path", attrs: (d: "m21 21-4.3-4.3"))
]

#let chevron-icon = _svg[
  #html.elem("path", attrs: (d: "m9 18 6-6-6-6"))
]

#let arrow-left-icon = _svg[
  #html.elem("path", attrs: (d: "M19 12H5M12 19l-7-7 7-7"))
]

#let arrow-right-icon = _svg[
  #html.elem("path", attrs: (d: "M5 12h14M12 5l7 7-7 7"))
]

#let link-icon = _svg[
  #html.elem("path", attrs: (d: "M10 13a5 5 0 0 0 7.1.1l2-2a5 5 0 0 0-7.1-7.1l-1.1 1.1M14 11a5 5 0 0 0-7.1-.1l-2 2A5 5 0 0 0 12 20l1.1-1.1"))
]

#let github-icon = _svg[
  #html.elem("path", attrs: (d: "M15 22v-4a4.8 4.8 0 0 0-1-3.5c3.3-.4 6.8-1.6 6.8-7A5.4 5.4 0 0 0 19.4 4 5 5 0 0 0 19.3.5S18.2.1 15 1.8a13.4 13.4 0 0 0-7 0C4.8.1 3.7.5 3.7.5A5 5 0 0 0 3.6 4a5.4 5.4 0 0 0-1.4 3.7c0 5.4 3.5 6.6 6.8 7A4.8 4.8 0 0 0 8 18v4M8 19c-3 .9-3-1.5-4-2"))
]

#let info-icon = _svg[
  #html.elem("circle", attrs: (cx: "12", cy: "12", r: "10"))
  #html.elem("path", attrs: (d: "M12 16v-4M12 8h.01"))
]

#let spark-icon = _svg[
  #html.elem("path", attrs: (d: "m12 3-1.9 5.1L5 10l5.1 1.9L12 17l1.9-5.1L19 10l-5.1-1.9L12 3Z"))
]

#let warning-icon = _svg[
  #html.elem("path", attrs: (d: "M10.3 2.9 1.8 17a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 2.9a2 2 0 0 0-3.4 0Z"))
  #html.elem("path", attrs: (d: "M12 9v4M12 17h.01"))
]
