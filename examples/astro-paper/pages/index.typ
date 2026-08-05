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

#metadata("./index.css") <aster-style>
#html.main(id: "main-content")[
  #html.header[
    #html.div[
      #html.h1[Mingalaba]
      #html.a(
        href: "/rss.xml",
        title: "RSS Feed",
        aria-label: "RSS Feed",
      )[#rss-icon]
    ]
    #html.p[
      AstroPaper is a minimal and responsive blog theme with accessible
      navigation, sensible SEO defaults, and built-in light and dark modes.
    ]
    #html.p[
      Read the blog posts or check the
      #link("https://github.com/satnaing/astro-paper#readme")[README]
      for more information.
    ]
    #html.aside[
      #html.span[Social Links:]
      #social-links()
    ]
  ]

  #if featured.len() > 0 [
    #html.section[
      #html.header[
        = Featured
      ]
      #post-list(featured.slice(0, calc.min(2, featured.len())), heading-level: 3)
    ]
  ]

  #html.section[
    #html.header[
      = Recent posts
      #link("/posts/")[All posts]
    ]
    #post-list(recent, heading-level: 3)
  ]
  #html.nav(aria-label: "All posts")[
    #html.a(href: "/posts/")[All Posts #arrow-right-icon]
  ]
]
