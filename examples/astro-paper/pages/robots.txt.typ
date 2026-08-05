#import "/lib.typ": settings

#let content = "User-agent: *\nAllow: /\n\nSitemap: " + settings.site.url + "sitemap.xml\n"

#metadata(content) <aster-endpoint>
