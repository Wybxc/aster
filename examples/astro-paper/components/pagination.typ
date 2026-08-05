#import "icons.typ": arrow-left-icon, arrow-right-icon

#let _href(base, page) = if page == 1 { base } else { base + str(page) + "/" }

#let pagination(page, total, base) = {
  if total > 1 [
    #metadata("./pagination.css") <aster-style>
    #html.elem("nav", attrs: ("aria-label": "Pagination"))[
      #if page > 1 {
        html.elem("a", attrs: (
          href: _href(base, page - 1),
          "aria-label": "Previous page",
        ))[#arrow-left-icon Previous]
      } else {
        html.elem("span", attrs: ("aria-disabled": "true"))[#arrow-left-icon Previous]
      }
      #html.elem("span", attrs: ("aria-current": "page"))[#page / #total]
      #if page < total {
        html.elem("a", attrs: (
          href: _href(base, page + 1),
          "aria-label": "Next page",
        ))[Next #arrow-right-icon]
      } else {
        html.elem("span", attrs: ("aria-disabled": "true"))[Next #arrow-right-icon]
      }
    ]
  ]
}
