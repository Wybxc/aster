#import "/lib.typ": doc-url
#import "icons.typ": arrow-left-icon, arrow-right-icon

#let pagination(adjacent) = [
  #metadata(
    ```css
    .pagination-links {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(min(18rem, 100%), 1fr));
      gap: 1rem;
      margin-top: 3rem;
    }

    .pagination-links a {
      display: flex;
      min-height: 5.75rem;
      align-items: center;
      gap: 0.65rem;
      border: 1px solid var(--sl-color-gray-5);
      border-radius: 0.5rem;
      padding: 1rem;
      color: var(--sl-color-gray-2);
      text-decoration: none;
      box-shadow: var(--sl-shadow-md);
    }

    .pagination-links a:hover {
      border-color: var(--sl-color-gray-2);
    }

    .pagination-links a[rel="next"] {
      justify-content: flex-end;
      text-align: end;
    }

    .pagination-links strong {
      display: block;
      margin-top: 0.1rem;
      color: var(--sl-color-white);
      font-size: 1.25rem;
      font-weight: 600;
      line-height: 1.2;
    }

    .pagination-links .sl-icon {
      width: 1.5rem;
      height: 1.5rem;
    }

    .pagination-links a > span[style] {
      display: none;
    }
    ```
  ) <aster-style>
  #html.nav(class: "pagination-links print-hidden", aria-label: "Page navigation")[
    #if adjacent.previous != none {
      let item = adjacent.previous
      html.a(href: doc-url(item.id), rel: "prev")[
        #arrow-left-icon
        #html.span[Previous #html.br() #html.strong[#item.metadata.title]]
      ]
    }
    #if adjacent.next != none {
      let item = adjacent.next
      html.a(href: doc-url(item.id), rel: "next")[
        #html.span[Next #html.br() #html.strong[#item.metadata.title]]
        #arrow-right-icon
      ]
    }
  ]
]
