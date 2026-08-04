#import "/lib.typ": published-posts, settings
#import "/components/icons.typ": arrow-right-icon, rss-icon
#import "/components/post-list.typ": post-list
#import "/components/social-links.typ": social-links
#import "/templates/site.typ": site

#let posts = published-posts()
#let featured = posts.filter(item => item.metadata.featured)
#let recent-posts = posts.filter(item => not item.metadata.featured)
#let recent = recent-posts.slice(0, calc.min(settings.posts.per-index, recent-posts.len()))

#show: site.with(path: "/")

#html.elem("main", attrs: (id: "main-content", class: "app-main home-main"))[
  #html.elem("section", attrs: (class: "intro"))[
    #html.elem("div", attrs: (class: "hero-title-row"))[
      #html.elem("h1")[Mingalaba]
      #html.elem("a", attrs: (
        href: "/rss.xml",
        class: "hero-rss",
        title: "RSS Feed",
        "aria-label": "RSS Feed",
      ))[#rss-icon]
    ]
    #html.elem("p", attrs: (class: "intro-copy"))[
      AstroPaper is a minimal and responsive blog theme with accessible
      navigation, sensible SEO defaults, and built-in light and dark modes.
    ]
    #html.elem("p", attrs: (class: "intro-secondary"))[
      Read the blog posts or check the
      #html.elem("a", attrs: (href: "https://github.com/satnaing/astro-paper#readme"))[README]
      for more information.
    ]
    #html.elem("div", attrs: (class: "home-socials"))[
      #html.elem("span")[Social Links:]
      #social-links()
    ]
  ]

  #if featured.len() > 0 [
    #html.elem("section", attrs: (class: "home-section"))[
      #html.elem("div", attrs: (class: "section-heading"))[
        #html.elem("h2")[Featured]
      ]
      #post-list(featured.slice(0, calc.min(2, featured.len())), heading-level: 3)
    ]
  ]

  #html.elem("section", attrs: (class: "home-section"))[
    #html.elem("div", attrs: (class: "section-heading"))[
      #html.elem("h2")[Recent posts]
      #html.elem("a", attrs: (href: "/posts/"))[All posts]
    ]
    #post-list(recent, heading-level: 3)
  ]
  #html.elem("div", attrs: (class: "all-posts"))[
    #html.elem("a", attrs: (href: "/posts/"))[All Posts #arrow-right-icon]
  ]
]
