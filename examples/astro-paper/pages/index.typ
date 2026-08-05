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

#metadata(
  ```css
  main {
    padding-bottom: 4rem;
  }

  main > header {
    min-height: 18rem;
    padding-block: 2.5rem;
  }

  main > header h1 {
    font-size: 2.25rem;
    font-weight: 700;
    line-height: 2.5rem;
  }

  main > header > div {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-block: 1rem;
  }

  main > header > div > a {
    display: inline-flex;
    width: 2.5rem;
    height: 2.5rem;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    text-decoration: none;
  }

  main > header > p:first-of-type {
    max-width: var(--measure);
    margin-top: 1.25rem;
    font-size: 1.125rem;
    line-height: 2rem;
  }

  main > header > p:last-of-type {
    margin-top: 0.5rem;
  }

  main > header > aside {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  main > section {
    border-top-width: 1px;
    padding-block: 2.25rem;
  }

  main > section > header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  main > section > header h2 {
    font-size: 1.5rem;
    font-weight: 600;
    line-height: 2rem;
  }

  main > nav {
    margin-block: 2rem;
    text-align: center;
  }

  main > nav > a {
    display: inline-flex;
    min-height: 2.75rem;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    font-weight: 500;
    text-decoration: none;
  }

  main > nav svg {
    width: 1.25rem;
    height: 1.25rem;
  }

  @media (min-width: 40rem) {
    main > header h1 {
      font-size: 3rem;
      line-height: 1;
    }

    main > header > div {
      margin-block: 2rem;
    }
  }

  @media (max-width: 639px) {
    main > header {
      min-height: 16rem;
      padding-block: 2rem;
    }

    main > header > aside {
      align-items: flex-start;
    }
  }
  ```
) <aster-style>

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
