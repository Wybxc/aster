#import "icons.typ": arrow-left-icon, arrow-right-icon

#let _href(base, page) = if page == 1 { base } else { base + str(page) + "/" }

#let pagination(page, total, base) = {
  if total > 1 [
    #metadata("./pagination.css") <aster-style>
    #html.nav(class: "pagination", aria-label: "Pagination")[
      #if page > 1 {
        html.a(
          href: _href(base, page - 1),
          aria-label: "Previous page",
        )[#arrow-left-icon Previous]
      } else {
        html.span(aria-disabled: true)[#arrow-left-icon Previous]
      }
      #html.span(aria-current: "page")[#page / #total]
      #if page < total {
        html.a(
          href: _href(base, page + 1),
          aria-label: "Next page",
        )[Next #arrow-right-icon]
      } else {
        html.span(aria-disabled: true)[Next #arrow-right-icon]
      }
    ]
  ]
}
