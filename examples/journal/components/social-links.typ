#import "/lib.typ": settings
#import "icons.typ": github-icon, linkedin-icon, mail-icon, x-icon

#let _icon(name) = {
  if name == "GitHub" {
    github-icon
  } else if name == "X" {
    x-icon
  } else if name == "LinkedIn" {
    linkedin-icon
  } else {
    mail-icon
  }
}

#let social-links() = {
  let items = settings.socials.map(social => {
    list.item(
      html.a(
        href: social.href,
        title: social.name,
        aria-label: social.name,
      )[#_icon(social.name)],
    )
  })
  [
    #metadata(
      ```css
      .social-links ul {
        display: flex;
        gap: 0.25rem;
      }

      .social-links a {
        position: relative;
        display: inline-flex;
        width: 2.5rem;
        height: 2.5rem;
        flex-shrink: 0;
        align-items: center;
        justify-content: center;
        background-color: transparent;
        padding: 0.5rem;
        color: var(--foreground);
        text-decoration: none;
      }

      .social-links a:hover {
        color: var(--accent);
      }
      ```
    ) <aster-style>
    #html.nav(class: "social-links", aria-label: "Social links")[
      #list(..items)
    ]
  ]
}
