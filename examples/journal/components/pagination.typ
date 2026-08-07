#import "icons.typ": arrow-left-icon, arrow-right-icon

#let _href(base, page) = if page == 1 { base } else { base + str(page) + "/" }

#let pagination(page, total, base) = {
  if total > 1 [
    #metadata(
      ```css
      .pagination {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 1rem;
        margin-top: 3rem;
      }

      .pagination > :is(a, span[aria-disabled="true"]) {
        display: inline-flex;
        min-width: 7rem;
        min-height: 2.75rem;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        padding-inline: 0.5rem;
        font-weight: 500;
        text-decoration: none;
      }

      .pagination span {
        color: var(--muted-foreground);
      }

      .pagination > span[aria-disabled="true"] {
        opacity: 0.5;
      }

      .pagination svg {
        width: 1.25rem;
        height: 1.25rem;
      }

      .pagination > span[aria-current="page"] {
        white-space: nowrap;
      }
      ```
    ) <aster-style>
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
