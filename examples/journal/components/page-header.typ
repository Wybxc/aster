#let page-header(title, description) = [
  #metadata(
    ```css
    .page-header h1 {
      font-size: 1.875rem;
      font-weight: 600;
      line-height: 2.25rem;
    }

    .page-header p {
      margin-top: 0.5rem;
      margin-bottom: 2rem;
      color: var(--muted-foreground);
      font-style: italic;
    }
    ```
  ) <aster-style>
  #html.header(class: "page-header")[
    #html.h1[#title]
    #html.p[#description]
  ]
]
