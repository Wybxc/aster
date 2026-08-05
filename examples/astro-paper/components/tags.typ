#import "/lib.typ": tag-slug

#let tag-list(items, counts: false, extra-class: none) = {
  let entries = items.map(item => {
    let name = if type(item) == str { item } else { item.name }
    let slug = if type(item) == str { tag-slug(item) } else { item.slug }
    list.item(
      html.a(href: "/tags/" + slug + "/")[
        #name
        #if counts { html.span[(#item.count)] }
      ],
    )
  })
  [
    #metadata(
      ```css
      .tag-list ul {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
      }

      .tag-list a {
        color: var(--accent);
        font-size: 0.875rem;
        line-height: 1.25rem;
        text-decoration-style: dashed;
      }

      .tag-list a::before {
        content: "#";
      }

      .tag-cloud a {
        display: inline-flex;
        min-height: 2.75rem;
        align-items: center;
        gap: 0.5rem;
        border-width: 1px;
        padding: 0.5rem 0.75rem;
        font-size: 1rem;
        line-height: 1.5rem;
        text-decoration: none;
      }

      .tag-cloud span {
        color: var(--muted-foreground);
      }
      ```
    ) <aster-style>
    #html.nav(
      class: (
        "tag-list"
        + if counts { " tag-cloud" } else { "" }
        + if extra-class == none { "" } else { " " + extra-class }
      ),
      aria-label: if counts { "All tags" } else { "Tags" },
    )[
      #list(..entries)
    ]
  ]
}
