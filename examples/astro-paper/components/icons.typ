#let _svg(body, class: "icon", view-box: "0 0 24 24", fill: "none") = {
  html.elem("svg", attrs: (
    xmlns: "http://www.w3.org/2000/svg",
    viewBox: view-box,
    fill: fill,
    stroke: "currentColor",
    "stroke-width": "2",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    class: class,
    "aria-hidden": "true",
  ))[#body]
}

#let menu-icon = _svg[
  #html.elem("path", attrs: (d: "M4 6h16M4 12h16M4 18h16"))
]

#let close-icon = _svg[
  #html.elem("path", attrs: (d: "M18 6 6 18M6 6l12 12"))
]

#let moon-icon = _svg[
  #html.elem("path", attrs: (d: "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79Z"))
]

#let sun-icon = _svg[
  #html.elem("circle", attrs: (cx: "12", cy: "12", r: "4"))
  #html.elem("path", attrs: (d: "M12 2v2M12 20v2M4.93 4.93l1.42 1.42M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.42-1.42M17.66 6.34l1.41-1.41"))
]

#let archive-icon = _svg[
  #html.elem("rect", attrs: (x: "3", y: "4", width: "18", height: "4", rx: "1"))
  #html.elem("path", attrs: (d: "M5 8v11a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8M10 12h4"))
]

#let rss-icon = _svg[
  #html.elem("path", attrs: (d: "M4 11a9 9 0 0 1 9 9M4 4a16 16 0 0 1 16 16"))
  #html.elem("circle", attrs: (cx: "5", cy: "19", r: "1"))
]

#let github-icon = _svg[
  #html.elem("path", attrs: (d: "M15 22v-4a4.8 4.8 0 0 0-1-3.5c3.3-.4 6.8-1.6 6.8-7A5.4 5.4 0 0 0 19.4 4 5 5 0 0 0 19.3.5S18.2.1 15 1.8a13.4 13.4 0 0 0-7 0C4.8.1 3.7.5 3.7.5A5 5 0 0 0 3.6 4a5.4 5.4 0 0 0-1.4 3.7c0 5.4 3.5 6.6 6.8 7A4.8 4.8 0 0 0 8 18v4M8 19c-3 .9-3-1.5-4-2"))
]

#let x-icon = html.elem("svg", attrs: (
  xmlns: "http://www.w3.org/2000/svg",
  viewBox: "0 0 24 24",
  fill: "currentColor",
  class: "icon",
  "aria-hidden": "true",
))[
  #html.elem("path", attrs: (d: "M18.9 2h3.7l-8.1 9.2L24 22h-7.4l-5.8-7.6L4.1 22H.4l8.7-9.9L0 2h7.6l5.2 6.9L18.9 2Zm-1.3 18.1h2L6.5 3.8H4.4l13.2 16.3Z"))
]

#let linkedin-icon = html.elem("svg", attrs: (
  xmlns: "http://www.w3.org/2000/svg",
  viewBox: "0 0 24 24",
  fill: "currentColor",
  class: "icon",
  "aria-hidden": "true",
))[
  #html.elem("path", attrs: (d: "M6.5 8.2H2.4V21h4.1V8.2ZM4.4 2A2.4 2.4 0 1 0 4.4 6.8 2.4 2.4 0 0 0 4.4 2ZM21.6 13.7c0-3.9-2.1-5.8-4.9-5.8-2.3 0-3.3 1.2-3.8 2.1V8.2H8.8V21h4.1v-6.3c0-1.7.3-3.3 2.4-3.3 2 0 2.1 1.9 2.1 3.4V21h4.2v-7.3Z"))
]

#let mail-icon = _svg[
  #html.elem("rect", attrs: (x: "3", y: "5", width: "18", height: "14", rx: "2"))
  #html.elem("path", attrs: (d: "m3 7 9 6 9-6"))
]

#let arrow-left-icon = _svg[
  #html.elem("path", attrs: (d: "M19 12H5M12 19l-7-7 7-7"))
]

#let arrow-right-icon = _svg[
  #html.elem("path", attrs: (d: "M5 12h14M12 5l7 7-7 7"))
]

#let arrow-up-icon = _svg[
  #html.elem("path", attrs: (d: "m18 15-6-6-6 6"))
]
