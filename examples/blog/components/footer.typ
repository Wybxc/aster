#import "/lib.typ": settings

#let footer() = [
  #metadata(
    ```css
    .site-footer {
      padding: 2rem 1rem 5rem;
      color: rgb(var(--gray));
      text-align: center;
    }

    .site-footer p { margin: 0.25rem; }
    .site-footer a { margin-inline: 0.35rem; }
    ```
  ) <aster-style>
  #html.footer(class: "site-footer")[
    #html.p[Copyright #datetime.today().display("[year]") #settings.site.author. All rights reserved.]
    #html.p[
      #link("https://github.com/Wybxc/aster")[Source]
      #link("mailto:hello@example.com")[Email]
    ]
  ]
]
